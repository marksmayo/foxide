//! A receiver-restricted, single-file HTTP origin. No directory or arbitrary-path serving.
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Owns one ephemeral URL. Dropping signals shutdown without blocking the UI.
/// Use shutdown() on a worker to wait for connections and file handles to close.
pub struct MediaOrigin {
    url: String,
    media_type: &'static str,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    wake_address: SocketAddr,
}
struct Resource {
    file: Mutex<File>,
    size: u64,
    route: String,
    authority: String,
    media_type: &'static str,
    stop: Arc<AtomicBool>,
}
impl MediaOrigin {
    pub fn new(path: &Path, receiver: IpAddr) -> io::Result<Self> {
        crate::transport::deadline(SocketAddr::new(receiver, 8009), Duration::from_secs(1))?;
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose an absolute media path",
            ));
        }
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let media_type = match extension.as_str() {
            "mp4" | "m4v" => "video/mp4",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            "mp3" => "audio/mpeg",
            "ogg" | "opus" => "audio/ogg",
            "flac" => "audio/flac",
            "wav" => "audio/wav",
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unsupported media container for casting",
                ));
            }
        };
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Choose a nonempty regular media file",
            ));
        }
        // UDP connect selects the interface/route without transmitting a datagram.
        let route_socket = UdpSocket::bind(if receiver.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        })?;
        route_socket.connect(SocketAddr::new(receiver, 8009))?;
        let listener = TcpListener::bind(SocketAddr::new(route_socket.local_addr()?.ip(), 0))?;
        let wake_address = listener.local_addr()?;
        let authority = wake_address.to_string();
        let mut token = [0; 32];
        getrandom::fill(&mut token)
            .map_err(|_| io::Error::other("Secure randomness unavailable"))?;
        let route = format!(
            "/foxide/{}",
            token.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        let url = format!("http://{authority}{route}");
        let stop = Arc::new(AtomicBool::new(false));
        let resource = Arc::new(Resource {
            file: Mutex::new(file),
            size: metadata.len(),
            route,
            authority,
            media_type,
            stop: stop.clone(),
        });
        let worker = thread::Builder::new()
            .name("foxide-cast-origin".into())
            .spawn(move || {
                let mut workers: Vec<JoinHandle<()>> = Vec::new();
                while !resource.stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((socket, peer)) => {
                            if resource.stop.load(Ordering::Acquire) {
                                break;
                            }
                            let mut index = 0;
                            while index < workers.len() {
                                if workers[index].is_finished() {
                                    let _ = workers.swap_remove(index).join();
                                } else {
                                    index += 1;
                                }
                            }

                            if peer.ip() != receiver || workers.len() >= 4 {
                                continue;
                            }
                            let resource = resource.clone();
                            if let Ok(worker) = thread::Builder::new()
                                .name("foxide-cast-http".into())
                                .spawn(move || {
                                    let _ = serve(socket, &resource);
                                })
                            {
                                workers.push(worker);
                            }
                        }
                        Err(_) => break,
                    }
                }
                drop(listener);
                for worker in workers {
                    let _ = worker.join();
                }
            })?;
        Ok(Self {
            url,
            media_type,
            stop,
            worker: Some(worker),
            wake_address,
        })
    }
    /// Contains a private address and bearer token; do not publish or log this URL.
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn media_type(&self) -> &'static str {
        self.media_type
    }
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        wake(self.wake_address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
// Wake a blocking accept without introducing a periodic listener/UI timer.
fn wake(address: SocketAddr) {
    let _ = TcpStream::connect_timeout(&address, Duration::from_millis(100));
}
impl Drop for MediaOrigin {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let address = self.wake_address;
            // Waiting for active clients belongs to cleanup, never the UI thread.
            if thread::Builder::new()
                .name("foxide-cast-cleanup".into())
                .spawn(move || {
                    wake(address);
                    let _ = worker.join();
                })
                .is_err()
            {
                // If a cleanup thread cannot be created, still wake the owner so
                // it can close its listener/files. This fallback is bounded.
                wake(self.wake_address);
            }
        }
    }
}

fn serve(mut socket: TcpStream, resource: &Resource) -> io::Result<()> {
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut header = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "HTTP request timed out"))?;
        socket.set_read_timeout(Some(remaining))?;
        let count = socket.read(&mut chunk)?;
        if count == 0 {
            return Ok(());
        }
        header.extend_from_slice(&chunk[..count]);
        if header.len() > 8192 {
            return error(&mut socket, "431 Request Header Fields Too Large");
        }
        if header.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if resource.stop.load(Ordering::Acquire) {
            return Ok(());
        }
    }
    if resource.stop.load(Ordering::Acquire) {
        return Ok(());
    }
    let Ok(header) = std::str::from_utf8(&header) else {
        return error(&mut socket, "400 Bad Request");
    };
    let mut lines = header.split("\r\n");
    let request: Vec<_> = lines.next().unwrap_or("").split(' ').collect();
    if request.len() != 3 || !matches!(request[2], "HTTP/1.0" | "HTTP/1.1") {
        return error(&mut socket, "400 Bad Request");
    }
    if request[1] != resource.route {
        return error(&mut socket, "404 Not Found");
    }
    let mut host = None;
    let mut range = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return error(&mut socket, "400 Bad Request");
        };
        if name.is_empty()
            || name
                .bytes()
                .any(|b| !b.is_ascii_alphanumeric() && b != b'-')
        {
            return error(&mut socket, "400 Bad Request");
        }
        if name.eq_ignore_ascii_case("host") {
            if host.replace(value.trim()).is_some() {
                return error(&mut socket, "400 Bad Request");
            }
        } else if name.eq_ignore_ascii_case("range") && range.replace(value.trim()).is_some() {
            return error(&mut socket, "400 Bad Request");
        }
    }
    if host.is_none_or(|h| !resource.authority.eq_ignore_ascii_case(h)) {
        return error(&mut socket, "400 Bad Request");
    }
    if request[0] == "OPTIONS" {
        return socket.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nAccess-Control-Allow-Headers: Range\r\nConnection: close\r\n\r\n");
    }
    if !matches!(request[0], "GET" | "HEAD") {
        return error(&mut socket, "405 Method Not Allowed");
    }
    let (start, end) = match range {
        None => (0, resource.size - 1),
        Some(value) => match byte_range(value, resource.size) {
            Some(range) => range,
            None => {
                write!(
                    socket,
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    resource.size
                )?;
                return Ok(());
            }
        },
    };
    let status = if range.is_some() {
        "206 Partial Content"
    } else {
        "200 OK"
    };
    write!(
        socket,
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\nCache-Control: no-store\r\nConnection: close\r\n",
        resource.media_type,
        end - start + 1
    )?;
    if range.is_some() {
        write!(
            socket,
            "Content-Range: bytes {start}-{end}/{}\r\n",
            resource.size
        )?;
    }
    socket.write_all(b"\r\n")?;
    if request[0] == "HEAD" {
        return Ok(());
    }
    let mut position = start;
    let mut buffer = [0; 64 * 1024];
    while position <= end && !resource.stop.load(Ordering::Acquire) {
        let length = usize::try_from((end - position + 1).min(buffer.len() as u64)).unwrap();
        let count = {
            // A shared open handle prevents path replacement and directory traversal.
            // Seek/read stay in one short critical section for concurrent range requests.
            let mut file = resource
                .file
                .lock()
                .map_err(|_| io::Error::other("Media unavailable"))?;
            file.seek(SeekFrom::Start(position))?;
            file.read(&mut buffer[..length])?
        };
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        socket.write_all(&buffer[..count])?;
        position += count as u64;
    }
    Ok(())
}
fn error(socket: &mut TcpStream, status: &str) -> io::Result<()> {
    write!(
        socket,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    let number = |value: &str| {
        if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
            value.parse::<u64>().ok()
        } else {
            None
        }
    };
    if start.is_empty() {
        let count = number(end)?;
        if count == 0 {
            return None;
        }
        return Some((size.saturating_sub(count), size - 1));
    }
    let start = number(start)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        number(end)?.min(size - 1)
    };
    (start <= end && start < size).then_some((start, end))
}
