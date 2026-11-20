use player_cast::MediaOrigin;
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct MediaFile {
    dir: PathBuf,
    path: PathBuf,
}
impl MediaFile {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "foxide-origin-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("selected movie, one.mp4");
        fs::write(&path, (0u8..=255).collect::<Vec<_>>()).unwrap();
        Self { dir, path }
    }
}
impl Drop for MediaFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.dir);
    }
}
fn request(url: &str, method: &str, extra: &str) -> Vec<u8> {
    let url = player_services::Url::parse(url).unwrap();
    let authority = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap());
    let mut stream = TcpStream::connect(&authority).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "{method} {} HTTP/1.1\r\nHost: {authority}\r\n{extra}\r\n",
        url.path()
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}
fn split(response: &[u8]) -> (&str, &[u8]) {
    let end = response.windows(4).position(|b| b == b"\r\n\r\n").unwrap() + 4;
    (
        std::str::from_utf8(&response[..end]).unwrap(),
        &response[end..],
    )
}
#[test]
fn shares_only_the_explicit_file_under_an_opaque_url_and_closes_it_on_shutdown() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let url = origin.url().to_string();
    assert!(!url.contains("selected"));
    let response = request(&url, "GET", "");
    let (headers, body) = split(&response);
    assert!(headers.starts_with("HTTP/1.1 200 "));
    assert!(headers.contains("Content-Type: video/mp4"));
    assert_eq!(body, (0u8..=255).collect::<Vec<_>>());
    let denied = request(&format!("{}/../anything", url), "GET", "");
    assert!(split(&denied).0.starts_with("HTTP/1.1 404 "));
    origin.shutdown();
    let url = player_services::Url::parse(&url).unwrap();
    assert!(TcpStream::connect((url.host_str().unwrap(), url.port().unwrap())).is_err());
}

#[test]
fn serves_correct_single_byte_ranges_and_head_without_reading_beyond_the_file() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    assert_eq!(origin.media_type(), "video/mp4");
    for (range, start, end) in [
        ("bytes=10-20", 10, 20),
        ("bytes=250-", 250, 255),
        ("bytes=-4", 252, 255),
        ("bytes=250-999", 250, 255),
        ("bytes=-999", 0, 255),
    ] {
        let response = request(origin.url(), "GET", &format!("Range: {range}\r\n"));
        let (headers, body) = split(&response);
        assert!(headers.starts_with("HTTP/1.1 206 "));
        assert!(headers.contains(&format!("Content-Range: bytes {start}-{end}/256")));
        assert_eq!(body, (start..=end).collect::<Vec<u8>>());
    }
    for range in [
        "bytes=256-",
        "bytes=20-10",
        "bytes=-0",
        "bytes=-",
        "bytes=1-2,4-5",
        "bytes=+1-3",
        "items=0-4",
        "bytes=18446744073709551616-",
        "bytes=1-x",
    ] {
        let response = request(origin.url(), "GET", &format!("Range: {range}\r\n"));
        let (headers, body) = split(&response);
        assert!(headers.starts_with("HTTP/1.1 416 "), "{range}: {headers}");
        assert!(headers.contains("Content-Range: bytes */256"));
        assert!(body.is_empty());
    }
    let response = request(origin.url(), "HEAD", "Range: bytes=10-20\r\n");
    let (headers, body) = split(&response);
    assert!(headers.starts_with("HTTP/1.1 206 "));
    assert!(headers.contains("Content-Length: 11"));
    assert!(body.is_empty());
    let response = request(origin.url(), "OPTIONS", "");
    assert!(split(&response).0.starts_with("HTTP/1.1 204 "));
    assert!(
        split(&response)
            .0
            .contains("Access-Control-Allow-Headers: Range")
    );
    origin.shutdown();
}
#[test]
fn rejects_wrong_methods_tokens_hosts_and_ambiguous_headers() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    assert!(
        split(&request(origin.url(), "POST", ""))
            .0
            .starts_with("HTTP/1.1 405 ")
    );
    for extra in [
        "Host: attacker.invalid\r\n",
        "Range: bytes=0-1\r\nRange: bytes=2-3\r\n",
        "Invalid header\r\n",
        " folded:value\r\n",
    ] {
        let response = request(origin.url(), "GET", extra);
        assert!(split(&response).0.starts_with("HTTP/1.1 400 "));
    }
    assert!(
        split(&request(&format!("{}wrong", origin.url()), "GET", ""))
            .0
            .starts_with("HTTP/1.1 404 ")
    );
    origin.shutdown();
}
#[test]
fn a_different_local_peer_cannot_fetch_even_with_the_token() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.2".parse().unwrap()).unwrap();
    let url = player_services::Url::parse(origin.url()).unwrap();
    let mut stream = TcpStream::connect((url.host_str().unwrap(), url.port().unwrap())).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let _ = write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        url.path(),
        url.host_str().unwrap(),
        url.port().unwrap()
    );
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    assert!(response.is_empty(), "unselected peer received a response");
    origin.shutdown();
}
#[test]
fn retains_the_selected_open_handle_when_the_path_is_replaced() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let original = media.dir.join("original.mp4");
    fs::rename(&media.path, &original).unwrap();
    fs::write(&media.path, b"this is a different file").unwrap();
    let response = request(origin.url(), "GET", "");
    assert_eq!(split(&response).1, (0u8..=255).collect::<Vec<_>>());
    origin.shutdown();
    fs::remove_file(original).unwrap();
}
#[test]
fn concurrent_seeks_keep_each_requested_range_intact() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let workers: Vec<_> = [0u8, 50, 100]
        .into_iter()
        .map(|start| {
            let url = origin.url().to_string();
            std::thread::spawn(move || {
                let response = request(
                    &url,
                    "GET",
                    &format!("Range: bytes={start}-{}\r\n", start + 20),
                );
                assert_eq!(split(&response).1, (start..=start + 20).collect::<Vec<_>>());
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    origin.shutdown();
}
#[test]
fn validates_file_and_receiver_before_starting_an_origin() {
    let media = MediaFile::new();
    assert!(
        MediaOrigin::new(
            std::path::Path::new("relative.mp4"),
            "127.0.0.1".parse().unwrap()
        )
        .is_err()
    );
    assert!(MediaOrigin::new(&media.path, "8.8.8.8".parse().unwrap()).is_err());
    fs::write(&media.path, []).unwrap();
    assert!(MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).is_err());
    fs::remove_file(&media.path).unwrap();
    assert!(MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).is_err());
    let bad = media.dir.join("file.exe");
    fs::write(&bad, [0]).unwrap();
    assert!(MediaOrigin::new(&bad, "127.0.0.1".parse().unwrap()).is_err());
    fs::remove_file(bad).unwrap();
}

fn raw_request(url: &str, bytes: &[u8]) -> Vec<u8> {
    let url = player_services::Url::parse(url).unwrap();
    let mut socket = TcpStream::connect((url.host_str().unwrap(), url.port().unwrap())).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    socket.write_all(bytes).unwrap();
    socket.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = Vec::new();
    if let Err(error) = socket.read_to_end(&mut response) {
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
    }
    response
}
#[test]
fn rejects_malformed_and_oversized_requests_without_echoing_inputs() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let path = player_services::Url::parse(origin.url())
        .unwrap()
        .path()
        .to_string();
    for request in [
        format!("GET {path} HTTP/2\r\n\r\n").into_bytes(),
        format!("GET {path} HTTP/1.1\r\n\r\n").into_bytes(),
        format!("GET  {path} HTTP/1.1\r\n\r\n").into_bytes(),
        [
            format!("GET {path} HTTP/1.1\r\nX: ").as_bytes(),
            &[255],
            b"\r\n\r\n",
        ]
        .concat(),
    ] {
        let response = raw_request(origin.url(), &request);
        assert!(split(&response).0.starts_with("HTTP/1.1 400 "));
        assert!(split(&response).1.is_empty());
    }
    let response = raw_request(
        origin.url(),
        format!("GET {path} HTTP/1.1\r\nX: {}\r\n\r\n", "x".repeat(9000)).as_bytes(),
    );
    assert!(split(&response).0.starts_with("HTTP/1.1 431 "));
    origin.shutdown();
}
#[test]
fn supports_the_declared_container_mime_types_with_fresh_unpredictable_routes() {
    let media = MediaFile::new();
    let mut paths = std::collections::HashSet::new();
    for (extension, mime) in [
        ("mp4", "video/mp4"),
        ("M4V", "video/mp4"),
        ("webm", "video/webm"),
        ("mkv", "video/x-matroska"),
        ("mp3", "audio/mpeg"),
        ("ogg", "audio/ogg"),
        ("opus", "audio/ogg"),
        ("flac", "audio/flac"),
        ("wav", "audio/wav"),
    ] {
        let path = media.dir.join(format!("file.{extension}"));
        fs::write(&path, [1]).unwrap();
        let origin = MediaOrigin::new(&path, "127.0.0.1".parse().unwrap()).unwrap();
        let url = player_services::Url::parse(origin.url()).unwrap();
        let token = url.path().strip_prefix("/foxide/").unwrap();
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(paths.insert(token.to_string()));
        let response = request(origin.url(), "HEAD", "");
        assert!(
            split(&response)
                .0
                .contains(&format!("Content-Type: {mime}"))
        );
        origin.shutdown();
        fs::remove_file(path).unwrap();
    }
}
#[test]
fn serves_multiple_chunks_and_shutdown_cancels_a_slow_transfer() {
    let media = MediaFile::new();
    let pattern = (0u8..=255).collect::<Vec<_>>();
    fs::write(&media.path, pattern.repeat(1024)).unwrap();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let response = request(origin.url(), "GET", "");
    assert_eq!(split(&response).1, pattern.repeat(1024));
    origin.shutdown();
    fs::write(&media.path, pattern.repeat(65536)).unwrap();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let url = player_services::Url::parse(origin.url()).unwrap();
    let mut socket = TcpStream::connect((url.host_str().unwrap(), url.port().unwrap())).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    write!(
        socket,
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\n\r\n",
        url.path(),
        url.host_str().unwrap(),
        url.port().unwrap()
    )
    .unwrap();
    let mut first = [0; 1024];
    assert!(socket.read(&mut first).unwrap() > 0);
    origin.shutdown();
    let mut tail = Vec::new();
    let _ = socket.read_to_end(&mut tail);
    assert!(
        tail.len() < 16 * 1024 * 1024,
        "shutdown left the entire slow transfer active"
    );
}

#[test]
fn dropping_an_origin_revokes_its_url_without_an_explicit_shutdown_call() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    let url = player_services::Url::parse(origin.url()).unwrap();
    let address = (url.host_str().unwrap(), url.port().unwrap());
    drop(origin);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while TcpStream::connect(address).is_ok() {
        assert!(
            std::time::Instant::now() < deadline,
            "dropped origin kept listening"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn reports_receiver_transport_progress_without_identifiers_or_urls() {
    let media = MediaFile::new();
    let origin = MediaOrigin::new(&media.path, "127.0.0.1".parse().unwrap()).unwrap();
    assert_eq!(origin.transfer_stats().bytes_sent, 0);
    let response = request(origin.url(), "GET", "");
    assert_eq!(split(&response).1.len(), 256);
    let stats = origin.transfer_stats();
    assert_eq!(stats.connections, 1);
    assert_eq!(stats.selected_peer_connections, 1);
    assert_eq!(stats.valid_requests, 1);
    assert_eq!(stats.bytes_sent, 256);
    origin.shutdown();
}
