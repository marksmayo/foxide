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
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done.load(Ordering::Acquire) && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut request = [0; 4096];
                let count = stream.read(&mut request).unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..count])
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
