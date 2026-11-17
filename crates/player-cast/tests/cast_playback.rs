use player_cast::{CastOptions, CastPlayback, CastState, CertificatePin};
use player_services::cast_wire::{Message, Payload};
use rustls::{ServerConfig, ServerConnection, StreamOwned, pki_types::PrivatePkcs8KeyDer};
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::Arc,
    thread,
    time::Duration,
};
const RECEIVER: &str = "urn:x-cast:com.google.cast.receiver";
const MEDIA: &str = "urn:x-cast:com.google.cast.media";
const CONNECTION: &str = "urn:x-cast:com.google.cast.tp.connection";
type Tls = StreamOwned<ServerConnection, TcpStream>;
struct FileFixture {
    dir: PathBuf,
    path: PathBuf,
}
impl FileFixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "foxide-cast-session-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("test movie.mp4");
        fs::write(&path, b"owned media bytes").unwrap();
        Self { dir, path }
    }
}
impl Drop for FileFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.dir);
    }
}
fn server(
    script: impl FnOnce(&mut Tls) -> io::Result<()> + Send + 'static,
) -> (
    SocketAddr,
    CertificatePin,
    thread::JoinHandle<io::Result<()>>,
) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pin = CertificatePin::of(key.cert.der());
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
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
        let (socket, _) = listener.accept()?;
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        socket.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut tls = StreamOwned::new(ServerConnection::new(Arc::new(config)).unwrap(), socket);
        script(&mut tls)
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    (address, pin, worker)
}
fn command(tls: &mut Tls, destination: &str, namespace: &str, kind: &str) -> io::Result<Value> {
    let message = Message::read(tls)?;
    assert_eq!(message.source(), "sender-0");
    assert_eq!(message.destination(), destination);
    assert_eq!(message.namespace(), namespace);
    let Payload::Json(value) = message.payload() else {
        panic!("expected JSON")
    };
    assert_eq!(value["type"], kind);
    Ok(value.clone())
}
fn reply(
    tls: &mut Tls,
    source: &str,
    namespace: &str,
    request: &Value,
    kind: &str,
    status: Value,
) -> io::Result<()> {
    let payload = if kind == "RECEIVER_STATUS" {
        json!({"type":kind,"requestId":request["requestId"],"status":status})
    } else {
        json!({"type":kind,"requestId":request["requestId"],"status":[status]})
    };
    Message::json(source, "sender-0", namespace, payload)?.write(tls)?;
    tls.flush()
}
fn apps(session: &str) -> Value {
    json!({"applications":[{"appId":"CC1AD845","sessionId":session,"transportId":"app-transport"}]})
}
fn media(url: &str, state: &str, time: f64) -> Value {
    json!({"mediaSessionId":7,"playerState":state,"currentTime":time,"media":{"contentId":url,"duration":20}})
}
fn begin(tls: &mut Tls) -> io::Result<String> {
    begin_with(tls, |_| {})
}
fn begin_with(tls: &mut Tls, transform: impl FnOnce(&mut Value)) -> io::Result<String> {
    begin_config(tls, json!({"applications":[]}), true, transform)
}
fn begin_config(
    tls: &mut Tls,
    current: Value,
    expected_autoplay: bool,
    transform: impl FnOnce(&mut Value),
) -> io::Result<String> {
    command(tls, "receiver-0", CONNECTION, "CONNECT")?;
    let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
    reply(
        tls,
        "receiver-0",
        RECEIVER,
        &query,
        "RECEIVER_STATUS",
        current,
    )?;
    let launch = command(tls, "receiver-0", RECEIVER, "LAUNCH")?;
    assert_eq!(launch["appId"], "CC1AD845");
    reply(
        tls,
        "receiver-0",
        RECEIVER,
        &launch,
        "RECEIVER_STATUS",
        apps("owned-session"),
    )?;
    command(tls, "app-transport", CONNECTION, "CONNECT")?;
    let load = command(tls, "app-transport", MEDIA, "LOAD")?;
    assert_eq!(load["media"]["contentType"], "video/mp4");
    assert_eq!(load["autoplay"], expected_autoplay);
    let url = load["media"]["contentId"].as_str().unwrap().to_string();
    // The receiver fetches the selected file through the real restricted HTTP origin.
    let parsed = player_services::Url::parse(&url).unwrap();
    let authority = format!("{}:{}", parsed.host_str().unwrap(), parsed.port().unwrap());
    let mut http = TcpStream::connect(&authority)?;
    http.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        http,
        "GET {} HTTP/1.1\r\nHost: {authority}\r\n\r\n",
        parsed.path()
    )?;
    let mut fetched = Vec::new();
    http.read_to_end(&mut fetched)?;
    assert!(fetched.ends_with(b"owned media bytes"));
    let mut loaded = media(&url, "PLAYING", 0.0);
    transform(&mut loaded);
    reply(tls, "app-transport", MEDIA, &load, "MEDIA_STATUS", loaded)?;
    Ok(url)
}
fn ownership(tls: &mut Tls, url: &str, state: &str, time: f64) -> io::Result<()> {
    let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
    reply(
        tls,
        "receiver-0",
        RECEIVER,
        &query,
        "RECEIVER_STATUS",
        apps("owned-session"),
    )?;
    let query = command(tls, "app-transport", MEDIA, "GET_STATUS")?;
    reply(
        tls,
        "app-transport",
        MEDIA,
        &query,
        "MEDIA_STATUS",
        media(url, state, time),
    )
}
#[test]
fn casts_the_selected_file_then_controls_only_its_owned_media_session() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        let url = begin(tls)?;
        ownership(tls, &url, "PLAYING", 1.0)?;
        let pause = command(tls, "app-transport", MEDIA, "PAUSE")?;
        assert_eq!(pause["mediaSessionId"], 7);
        reply(
            tls,
            "app-transport",
            MEDIA,
            &pause,
            "MEDIA_STATUS",
            media(&url, "PAUSED", 1.0),
        )?;
        ownership(tls, &url, "PAUSED", 1.0)?;
        let seek = command(tls, "app-transport", MEDIA, "SEEK")?;
        assert_eq!(seek["currentTime"], 10.0);
        reply(
            tls,
            "app-transport",
            MEDIA,
            &seek,
            "MEDIA_STATUS",
            media(&url, "PAUSED", 10.0),
        )?;
        ownership(tls, &url, "PAUSED", 10.0)?;
        let play = command(tls, "app-transport", MEDIA, "PLAY")?;
        reply(
            tls,
            "app-transport",
            MEDIA,
            &play,
            "MEDIA_STATUS",
            media(&url, "PLAYING", 10.0),
        )?;
        ownership(tls, &url, "PLAYING", 10.0)?;
        let stop = command(tls, "app-transport", MEDIA, "STOP")?;
        assert_eq!(stop["mediaSessionId"], 7);
        reply(
            tls,
            "app-transport",
            MEDIA,
            &stop,
            "MEDIA_STATUS",
            media(&url, "IDLE", 10.0),
        )?;
        Ok(())
    });
    let mut playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(playback.last_status().state, CastState::Playing);
    assert_eq!(playback.pause().unwrap().state, CastState::Paused);
    for time in [f64::NAN, f64::INFINITY, -1.0, 21.0] {
        assert!(playback.seek(time).is_err());
    }
    assert_eq!(playback.seek(10.0).unwrap().position, 10.0);
    assert_eq!(playback.play().unwrap().state, CastState::Playing);
    playback.stop().unwrap();
    worker.join().unwrap().unwrap();
}
#[test]
fn refuses_to_replace_an_existing_application_without_explicit_takeover() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        command(tls, "receiver-0", CONNECTION, "CONNECT")?;
        let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
        reply(
            tls,
            "receiver-0",
            RECEIVER,
            &query,
            "RECEIVER_STATUS",
            json!({"applications":[{"appId":"someone-else"}]}),
        )?;
        assert!(
            Message::read(tls).is_err(),
            "sent a command after refusing takeover"
        );
        Ok(())
    });
    assert_eq!(
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default())
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    worker.join().unwrap().unwrap();
}
#[test]
fn loses_control_when_another_application_replaces_the_owned_receiver_session() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin(tls)?;
        let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
        reply(
            tls,
            "receiver-0",
            RECEIVER,
            &query,
            "RECEIVER_STATUS",
            apps("someone-elses-session"),
        )?;
        assert!(
            Message::read(tls).is_err(),
            "sent media commands after losing ownership"
        );
        Ok(())
    });
    let mut playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(
        playback.pause().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    drop(playback);
    worker.join().unwrap().unwrap();
}

#[test]
fn refuses_a_malformed_playback_clock_instead_of_reporting_zero() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin_with(tls, |status| status["currentTime"] = json!("invalid"))?;
        Ok(())
    });
    let result = CastPlayback::start_file(&file.path, address, pin, CastOptions::default());
    assert!(
        result.is_err(),
        "invented a valid zero-second clock for malformed telemetry"
    );
    worker.join().unwrap().unwrap();
}

#[test]
fn never_stops_another_senders_media_inside_the_same_application() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin(tls)?;
        ownership(tls, "another-senders-media", "PLAYING", 1.0)?;
        assert!(
            Message::read(tls).is_err(),
            "STOP reached a media session owned by another sender"
        );
        Ok(())
    });
    let playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(
        playback.stop().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    worker.join().unwrap().unwrap();
}

#[test]
fn receiver_launch_rejection_never_exposes_a_media_url_or_sends_load() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        command(tls, "receiver-0", CONNECTION, "CONNECT")?;
        let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
        reply(
            tls,
            "receiver-0",
            RECEIVER,
            &query,
            "RECEIVER_STATUS",
            json!({}),
        )?;
        let launch = command(tls, "receiver-0", RECEIVER, "LAUNCH")?;
        Message::json(
            "receiver-0",
            "sender-0",
            RECEIVER,
            json!({"type":"LAUNCH_ERROR","requestId":launch["requestId"],"reason":"NOT_FOUND"}),
        )?
        .write(tls)?;
        tls.flush()?;
        assert!(
            Message::read(tls).is_err(),
            "LOAD followed a rejected launch"
        );
        Ok(())
    });
    assert!(CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).is_err());
    worker.join().unwrap().unwrap();
}

#[test]
fn accepts_partial_status_for_the_owned_media_and_preserves_known_duration() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin(tls)?;
        let query = command(tls, "receiver-0", RECEIVER, "GET_STATUS")?;
        reply(
            tls,
            "receiver-0",
            RECEIVER,
            &query,
            "RECEIVER_STATUS",
            apps("owned-session"),
        )?;
        let query = command(tls, "app-transport", MEDIA, "GET_STATUS")?;
        reply(
            tls,
            "app-transport",
            MEDIA,
            &query,
            "MEDIA_STATUS",
            json!({"mediaSessionId":7,"playerState":"PLAYING","currentTime":1}),
        )?;
        let pause = command(tls, "app-transport", MEDIA, "PAUSE")?;
        reply(
            tls,
            "app-transport",
            MEDIA,
            &pause,
            "MEDIA_STATUS",
            json!({"mediaSessionId":7,"playerState":"PAUSED","currentTime":1}),
        )?;
        Ok(())
    });
    let mut playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    let status = playback.pause().unwrap();
    assert_eq!(status.state, CastState::Paused);
    assert_eq!(status.duration, Some(20.0));
    drop(playback);
    worker.join().unwrap().unwrap();
}

#[test]
fn surfaces_invalid_player_state_as_receiver_rejection() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        let url = begin(tls)?;
        ownership(tls, &url, "PLAYING", 0.0)?;
        let pause = command(tls, "app-transport", MEDIA, "PAUSE")?;
        Message::json(
            "app-transport",
            "sender-0",
            MEDIA,
            json!({"type":"INVALID_PLAYER_STATE","requestId":pause["requestId"]}),
        )?
        .write(tls)?;
        tls.flush()
    });
    let mut playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(playback.pause().unwrap_err().kind(), io::ErrorKind::Other);
    drop(playback);
    worker.join().unwrap().unwrap();
}

#[test]
fn can_launch_from_an_explicit_receiver_idle_screen_without_taking_over_content() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin_config(
            tls,
            json!({"applications":[{"appId":"idle-app","isIdleScreen":true}]}),
            true,
            |_| {},
        )?;
        Ok(())
    });
    let playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(playback.last_status().state, CastState::Playing);
    drop(playback);
    worker.join().unwrap().unwrap();
}

#[test]
fn waits_for_confirmed_playback_after_the_receivers_intermediate_loading_status() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        let url = begin_with(tls, |value| {
            value["playerState"] = json!("IDLE");
            value["extendedStatus"] = json!({"playerState":"LOADING"});
        })?;
        Message::json(
            "app-transport",
            "sender-0",
            MEDIA,
            json!({"type":"MEDIA_STATUS","requestId":0,"status":[media(&url,"PLAYING",0.3)]}),
        )?
        .write(tls)?;
        tls.flush()
    });
    let playback =
        CastPlayback::start_file(&file.path, address, pin, CastOptions::default()).unwrap();
    assert_eq!(playback.last_status().state, CastState::Playing);
    drop(playback);
    worker.join().unwrap().unwrap();
}

#[test]
fn explicit_takeover_and_paused_start_require_the_requested_options() {
    let file = FileFixture::new();
    let (address, pin, worker) = server(|tls| {
        begin_config(
            tls,
            json!({"applications":[{"appId":"other-app","isIdleScreen":false}]}),
            false,
            |value| value["playerState"] = json!("PAUSED"),
        )?;
        Ok(())
    });
    let options = CastOptions {
        allow_replacing_application: true,
        initially_paused: true,
        ..CastOptions::default()
    };
    let playback = CastPlayback::start_file(&file.path, address, pin, options).unwrap();
    assert_eq!(playback.last_status().state, CastState::Paused);
    drop(playback);
    worker.join().unwrap().unwrap();
}
