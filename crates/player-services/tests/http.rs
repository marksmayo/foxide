use player_services::{CurlClient, HttpClient};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[test]
fn system_http_client_fetches_a_bounded_local_response_without_a_shell() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !done.load(Ordering::Acquire) && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                // Winsock may inherit nonblocking mode from the listener.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 512];
                while !request.ends_with(b"\r\n\r\n") && request.len() < 4096 {
                    let count = stream.read(&mut chunk).unwrap();
                    assert!(count > 0, "Client closed before sending request headers");
                    request.extend_from_slice(&chunk[..count]);
                }
                assert!(
                    String::from_utf8_lossy(&request)
                        .starts_with("GET /fixture?name=jazz%20%26%20soul HTTP/")
                );
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
                stream.write_all(br#"{"value":123}"#).unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let client = CurlClient::system().unwrap();
    let url = url::Url::parse(&format!("http://{address}/fixture?name=jazz%20%26%20soul")).unwrap();
    let result = client.get(&url, &AtomicBool::new(false));
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    assert_eq!(result.unwrap(), br#"{"value":123}"#);
}
