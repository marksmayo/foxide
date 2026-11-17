use player_cast::CertificatePin;

#[test]
fn certificate_pins_round_trip_and_reject_ambiguous_input() {
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let pin = CertificatePin::of(b"abc");
    assert_eq!(pin.to_string(), expected);
    assert_eq!(expected.parse::<CertificatePin>().unwrap(), pin);
    assert_eq!(
        expected.to_uppercase().parse::<CertificatePin>().unwrap(),
        pin
    );
    for bad in [
        "",
        "00",
        &"x".repeat(64),
        &format!(" {expected}"),
        &"é".repeat(32),
    ] {
        assert!(bad.parse::<CertificatePin>().is_err());
    }
}

use player_cast::receiver_status;
use player_services::cast_wire::{Message, Payload};
use rustls::{ServerConfig, ServerConnection, StreamOwned, pki_types::PrivatePkcs8KeyDer};
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    thread,
    time::Duration,
};

const CONNECTION: &str = "urn:x-cast:com.google.cast.tp.connection";
const RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";

type FixtureStream = StreamOwned<ServerConnection, std::net::TcpStream>;
fn fixture(
    version: &'static rustls::SupportedProtocolVersion,
    respond: impl FnOnce(&mut FixtureStream) -> std::io::Result<()> + Send + 'static,
) -> (
    SocketAddr,
    CertificatePin,
    thread::JoinHandle<Result<(), String>>,
) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pin = CertificatePin::of(key.cert.der());
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[version])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![key.cert.der().clone()],
                PrivatePkcs8KeyDer::from(key.signing_key.serialize_der()).into(),
            )
            .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(0);
    let worker = thread::spawn(move || {
        ready_tx.send(()).unwrap();
        let (socket, _) = listener.accept().map_err(|e| e.to_string())?;
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut tls = StreamOwned::new(ServerConnection::new(Arc::new(config)).unwrap(), socket);
        let connect = Message::read(&mut tls).map_err(|e| e.to_string())?;
        assert_eq!(connect.namespace(), CONNECTION);
        assert_eq!(connect.destination(), "receiver-0");
        assert_eq!(
            connect.payload(),
            &Payload::Json(serde_json::json!({"type":"CONNECT", "origin":{}}))
        );
        let status = Message::read(&mut tls).map_err(|e| e.to_string())?;
        assert_eq!(status.namespace(), RECEIVER);
        assert_eq!(
            status.payload(),
            &Payload::Json(serde_json::json!({"type":"GET_STATUS","requestId":1}))
        );
        respond(&mut tls).map_err(|e| e.to_string())
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    (address, pin, worker)
}
fn send(
    tls: &mut FixtureStream,
    source: &str,
    dest: &str,
    ns: &str,
    value: serde_json::Value,
) -> std::io::Result<()> {
    Message::json(source, dest, ns, value)?.write(tls)?;
    std::io::Write::flush(tls)
}
fn good_status(tls: &mut FixtureStream) -> std::io::Result<()> {
    send(
        tls,
        "receiver-0",
        "sender-0",
        RECEIVER,
        serde_json::json!({"type":"RECEIVER_STATUS", "requestId":1, "status":{"applications":[{"appId":"test-app"}]}}),
    )
}
fn receiver_fixture() -> (
    SocketAddr,
    CertificatePin,
    thread::JoinHandle<Result<(), String>>,
) {
    fixture(&rustls::version::TLS13, good_status)
}

#[test]
fn reads_receiver_status_only_after_the_pinned_tls_handshake() {
    let (address, pin, worker) = receiver_fixture();
    let status = receiver_status(address, pin, Duration::from_secs(5)).unwrap();
    assert_eq!(status["applications"][0]["appId"], "test-app");
    worker.join().unwrap().unwrap();
}

#[test]
fn refuses_a_changed_certificate_before_sending_application_commands() {
    let (address, _, worker) = receiver_fixture();
    let wrong = CertificatePin::of(b"another receiver certificate");
    assert!(receiver_status(address, wrong, Duration::from_secs(5)).is_err());
    assert!(
        worker.join().unwrap().is_err(),
        "untrusted peer received application data"
    );
}

#[test]
fn certificate_inspection_never_sends_application_commands_or_grants_trust() {
    let (address, pin, worker) = receiver_fixture();
    let inspected =
        player_cast::inspect_receiver_certificate(address, Duration::from_secs(5)).unwrap();
    assert_eq!(inspected, pin);
    assert!(
        worker.join().unwrap().is_err(),
        "inspection sent application data"
    );
}

#[test]
fn supports_tls12_receivers_without_disabling_handshake_signature_checks() {
    let (address, pin, worker) = fixture(&rustls::version::TLS12, good_status);
    assert!(receiver_status(address, pin, Duration::from_secs(5)).is_ok());
    worker.join().unwrap().unwrap();
}
#[test]
fn answers_heartbeats_while_ignoring_unrelated_messages() {
    let (address, pin, worker) = fixture(&rustls::version::TLS13, |tls| {
        let heartbeat = "urn:x-cast:com.google.cast.tp.heartbeat";
        send(
            tls,
            "receiver-0",
            "sender-0",
            heartbeat,
            serde_json::json!({"type":"PING"}),
        )?;
        let reply = Message::read(tls)?;
        assert_eq!(reply.namespace(), heartbeat);
        assert_eq!(
            reply.payload(),
            &Payload::Json(serde_json::json!({"type":"PONG"}))
        );
        for (src, dest, ns, value) in [
            ("other", "sender-0", RECEIVER, serde_json::json!({})),
            ("receiver-0", "other", RECEIVER, serde_json::json!({})),
            ("receiver-0", "sender-0", "urn:x", serde_json::json!({})),
            (
                "receiver-0",
                "sender-0",
                RECEIVER,
                serde_json::json!({"type":"RECEIVER_STATUS","requestId":2,"status":{}}),
            ),
            (
                "receiver-0",
                "sender-0",
                RECEIVER,
                serde_json::json!({"type":"UNEXPECTED","requestId":1}),
            ),
        ] {
            send(tls, src, dest, ns, value)?;
        }
        Message::binary("receiver-0", "sender-0", "urn:x", vec![0])?.write(tls)?;
        good_status(tls)
    });
    assert!(receiver_status(address, pin, Duration::from_secs(5)).is_ok());
    worker.join().unwrap().unwrap();
}
#[test]
fn accepts_a_broadcast_status_only_with_the_expected_request_id() {
    let (address, pin, worker) = fixture(&rustls::version::TLS13, |tls| {
        send(
            tls,
            "receiver-0",
            "*",
            RECEIVER,
            serde_json::json!({"type":"RECEIVER_STATUS","requestId":1,"status":{}}),
        )
    });
    assert_eq!(
        receiver_status(address, pin, Duration::from_secs(5)).unwrap(),
        serde_json::json!({})
    );
    worker.join().unwrap().unwrap();
}
#[test]
fn rejects_malformed_or_excessive_responses() {
    for status in [serde_json::json!(null), serde_json::json!([])] {
        let (address, pin, worker) = fixture(&rustls::version::TLS13, move |tls| {
            send(
                tls,
                "receiver-0",
                "sender-0",
                RECEIVER,
                serde_json::json!({"type":"RECEIVER_STATUS","requestId":1,"status":status}),
            )
        });
        assert!(receiver_status(address, pin, Duration::from_secs(5)).is_err());
        worker.join().unwrap().unwrap();
    }
    let (address, pin, worker) = fixture(&rustls::version::TLS13, |tls| {
        for _ in 0..64 {
            send(tls, "other", "sender-0", RECEIVER, serde_json::json!({}))?;
        }
        Ok(())
    });
    assert_eq!(
        receiver_status(address, pin, Duration::from_secs(5))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    worker.join().unwrap().unwrap();
}
#[test]
fn rejects_public_unspecified_and_invalid_receiver_requests_before_connecting() {
    let pin = CertificatePin::of(b"fixture");
    for addr in [
        "8.8.8.8:8009",
        "0.0.0.0:8009",
        "224.0.0.1:8009",
        "[2001:4860:4860::8888]:8009",
        "[::]:8009",
        "127.0.0.1:0",
    ] {
        let addr = addr.parse().unwrap();
        assert_eq!(
            receiver_status(addr, pin, Duration::from_secs(1))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
        assert_eq!(
            player_cast::inspect_receiver_certificate(addr, Duration::from_secs(1))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    for timeout in [Duration::ZERO, Duration::from_secs(31)] {
        assert_eq!(
            receiver_status("127.0.0.1:8009".parse().unwrap(), pin, timeout)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    // Local IPv6 is accepted by validation even when no receiver is listening.
    assert_ne!(
        receiver_status("[::1]:1".parse().unwrap(), pin, Duration::from_millis(1))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}
#[test]
fn missing_or_non_tls_receivers_fail_without_returning_a_trusted_pin() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    assert!(player_cast::inspect_receiver_certificate(address, Duration::from_secs(1)).is_err());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        use std::io::Write;
        let (mut socket, _) = listener.accept().unwrap();
        socket.write_all(b"this is not TLS").unwrap();
    });
    assert!(player_cast::inspect_receiver_certificate(address, Duration::from_secs(1)).is_err());
    worker.join().unwrap();
}
#[test]
fn absolute_deadline_stops_a_trickled_frame() {
    let (address, pin, worker) = fixture(&rustls::version::TLS13, |tls| {
        use std::io::Write;
        let mut frame = Vec::new();
        Message::json(
            "receiver-0",
            "sender-0",
            RECEIVER,
            serde_json::json!({"padding":"x".repeat(1024)}),
        )?
        .write(&mut frame)?;
        for byte in frame {
            if tls.write_all(&[byte]).and_then(|_| tls.flush()).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        Ok(())
    });
    let failure = receiver_status(address, pin, Duration::from_secs(2)).unwrap_err();
    assert!(
        matches!(
            failure.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ),
        "{failure}"
    );
    worker.join().unwrap().unwrap();
}
