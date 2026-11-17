//! Explicit helper -> built-in mpv hook -> HTTP headers -> actual video/audio decoders.
use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    process::Command,
    time::Instant,
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "Requires libmpv built with the Lua ytdl hook and a Rust compiler"]
fn video_site_hook_uses_only_the_selected_helper_and_preserves_stream_headers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../results/local/native-sites");
    let dir = Scratch(root.join(format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir_all(&dir.0).unwrap();
    let dir_path = dir.0.canonicalize().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let args_log = dir_path.join("helper-args.txt");
    let helper = dir_path.join(if cfg!(windows) {
        "yt dlp,fixture.exe"
    } else {
        "yt dlp,fixture"
    });
    let headers = serde_json::json!({"Referer":"https://example.org/fixture-reference"});
    let metadata = serde_json::json!({"id":"fixture", "title":"Foxide video-site fixture", "extractor":"generic", "requested_formats":[
        {"url":format!("http://{address}/video.y4m"), "ext":"y4m", "protocol":"http", "format_id":"video", "vcodec":"rawvideo", "acodec":"none", "http_headers":headers},
        {"url":format!("http://{address}/audio.wav"), "ext":"wav", "protocol":"http", "format_id":"audio", "vcodec":"none", "acodec":"pcm_s16le", "http_headers":headers}
    ]}).to_string();
    let source = format!(
        "fn main() {{ std::fs::write({:?}, std::env::args().collect::<Vec<_>>().join(\"\\n\")).unwrap(); println!(\"{{}}\", {:?}); }}",
        args_log.to_str().unwrap(),
        metadata
    );
    let fixture_source = dir_path.join("helper.rs");
    std::fs::write(&fixture_source, source).unwrap();
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg(&fixture_source)
        .arg("-o")
        .arg(&helper)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let probe = Command::new(&helper).output().unwrap();
    assert!(
        probe.status.success(),
        "Selected helper must execute directly"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&probe.stdout).unwrap(),
        serde_json::from_str::<serde_json::Value>(&metadata).unwrap()
    );
    std::fs::remove_file(&args_log).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let finished = stopped.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !finished.load(Ordering::Acquire) && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 512];
                while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request).into_owned());
                let pcm = vec![0u8; 32000];
                let mut wav = Vec::new();
                wav.extend_from_slice(b"RIFF");
                wav.extend_from_slice(&(36u32 + pcm.len() as u32).to_le_bytes());
                wav.extend_from_slice(b"WAVEfmt ");
                wav.extend_from_slice(&16u32.to_le_bytes());
                wav.extend_from_slice(&1u16.to_le_bytes());
                wav.extend_from_slice(&1u16.to_le_bytes());
                wav.extend_from_slice(&8000u32.to_le_bytes());
                wav.extend_from_slice(&16000u32.to_le_bytes());
                wav.extend_from_slice(&2u16.to_le_bytes());
                wav.extend_from_slice(&16u16.to_le_bytes());
                wav.extend_from_slice(b"data");
                wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
                wav.extend(pcm);
                let is_video = request.starts_with(b"GET /video.y4m ");
                let body = if is_video {
                    let mut video = b"YUV4MPEG2 W2 H2 F25:1 Ip A1:1 C420jpeg\n".to_vec();
                    for _ in 0..25 {
                        video.extend_from_slice(b"FRAME\n");
                        video.extend_from_slice(&[16, 16, 16, 16, 128, 128]);
                    }
                    video
                } else {
                    wav
                };
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream
                    .write_all(header.as_bytes())
                    .and_then(|_| stream.write_all(&body));
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }
    });
    let tool = player_services::VideoSiteTool::new(&helper).unwrap();
    let api = Api::load(Some(&tool)).unwrap();
    for args in [
        ["set", "vo", "null"],
        ["set", "ao", "null"],
        ["set", "pause", "no"],
    ] {
        api.send(&args.map(str::to_owned)).unwrap();
    }
    api.send(&[
        "loadfile".into(),
        "ytdl://https://www.youtube.com/watch?v=foxide-fixture".into(),
        "replace".into(),
    ])
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut loaded = false;
    let mut restarted = false;
    let mut diagnostics = Vec::new();
    while Instant::now() < deadline {
        // SAFETY: event data is consumed before the next wait_event; this test owns the client.
        unsafe {
            let event = (api.wait_event)(api.handle, 0.05);
            if event.is_null() {
                continue;
            }
            if (*event).kind == 8 {
                loaded = true;
            }
            if (*event).kind == 21 {
                restarted = true;
            }
            if loaded && restarted {
                break;
            }
            if (*event).kind == 2 && !(*event).data.is_null() {
                let message = &*((*event).data.cast::<LogMessage>());
                if !message.text.is_null() {
                    diagnostics.push(CStr::from_ptr(message.text).to_string_lossy().into_owned());
                }
            }
            if (*event).kind == 7 {
                break;
            }
        }
    }
    stopped.store(true, Ordering::Release);
    server.join().unwrap();
    assert!(
        loaded && restarted,
        "Native hook did not load and decode fixture: {}",
        diagnostics.join("")
    );
    assert_eq!(api.get(c"audio-codec-name"), "pcm_s16le");
    assert_eq!(api.get(c"video-format"), "rawvideo");
    let args = std::fs::read_to_string(args_log).unwrap();
    assert!(args.contains("https://www.youtube.com/watch?v=foxide-fixture"));
    assert!(
        requests.lock().unwrap().iter().any(|request| request
            .to_ascii_lowercase()
            .contains("referer: https://example.org/fixture-reference")),
        "HTTP headers were lost"
    );
}

fn null_output_engine() -> Engine {
    let engine = Engine::new().unwrap();
    engine
        .command(vec!["set".into(), "vo".into(), "null".into()])
        .unwrap();
    engine
        .command(vec!["set".into(), "ao".into(), "null".into()])
        .unwrap();
    engine.ready.store(true, Ordering::Release);
    engine
}
fn load_fixture(engine: &Engine, name: &str) -> Snapshot {
    let media = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    engine
        .command(vec![
            "loadfile".into(),
            media.to_str().unwrap().into(),
            "replace".into(),
        ])
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut snapshot = Snapshot::default();
    while Instant::now() < deadline {
        snapshot = engine.snapshot().unwrap();
        if !snapshot.error.is_empty()
            || (snapshot.loaded
                && snapshot.path.ends_with(name)
                && snapshot.tracks.iter().any(|track| track.kind == "audio"))
        {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(snapshot.error.is_empty(), "{}", snapshot.error);
    assert!(
        snapshot.loaded && snapshot.path.ends_with(name),
        "Requested fixture must load before metadata is accepted"
    );
    snapshot
}

#[test]
#[ignore = "Requires libmpv; exercises real decoding with null output and no window"]
fn selected_video_tracks_reach_the_frontend_metadata() {
    let engine = null_output_engine();
    let snapshot = load_fixture(&engine, "tracks.mkv");
    assert!(
        snapshot
            .tracks
            .iter()
            .any(|track| track.kind == "video" && track.selected),
        "A selected video track must enable video-only frontend controls"
    );
}

#[test]
#[ignore = "Requires libmpv; validates the selected embedded artwork track"]
fn embedded_artwork_is_separate_from_video_controls_and_clears_on_file_change() {
    let engine = null_output_engine();
    let artwork = load_fixture(&engine, "artwork.flac");
    assert!(
        artwork.has_artwork,
        "The selected embedded picture must be exposed for the music view"
    );
    assert!(
        !artwork
            .tracks
            .iter()
            .any(|track| track.kind == "video" && track.selected),
        "Attached cover art must not enable video-only frame controls"
    );
    let video = load_fixture(&engine, "tracks.mkv");
    assert!(
        !video.has_artwork,
        "Previous artwork must not survive a file change"
    );
    assert!(
        video
            .tracks
            .iter()
            .any(|track| track.kind == "video" && track.selected)
    );
}
