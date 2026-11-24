#![cfg(feature = "desktop")]
use crate::{Action, Effect, UiState};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Probe {
    directory: PathBuf,
    child: Option<Child>,
}
impl Drop for Probe {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
        assert_eq!(self.directory.parent(), Some(root.as_path()));
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
#[ignore = "Requires explicit FOXIDE_MPV and FOXIDE_CONTROL_FIXTURE"]
fn desktop_menu_commands_are_accepted_by_the_native_media_engine() {
    let executable =
        std::env::var_os("FOXIDE_MPV").expect("Set FOXIDE_MPV to a trusted mpv executable");
    let fixture = std::env::var_os("FOXIDE_CONTROL_FIXTURE")
        .expect("Set FOXIDE_CONTROL_FIXTURE to a video with audio");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
    fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!(
        "controls-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let mut probe = Probe {
        directory,
        child: None,
    };
    let mut state = UiState::default();
    state.confirm_loaded(10.0);
    let scenarios = [
        ("rotation", 90.0, "video-rotate", json!(90)),
        ("rotation", 0.0, "video-rotate", json!(0)),
        ("flip-vertical", 0.0, "", json!(null)),
        ("flip-vertical", 0.0, "", json!(null)),
        ("flip-horizontal", 0.0, "", json!(null)),
        ("flip-horizontal", 0.0, "", json!(null)),
        ("replaygain", 2.0, "replaygain", json!("album")),
        ("replaygain", 1.0, "replaygain", json!("track")),
        ("replaygain", 0.0, "replaygain", json!(false)),
        ("secondary-subtitle", 2.0, "secondary-sid", json!(2)),
        ("secondary-subtitle", 0.0, "secondary-sid", json!(false)),
        ("deband", 1.0, "deband", json!(true)),
        ("deband", 0.0, "deband", json!(false)),
        ("equalizer", 1.0, "", json!(null)),
        ("equalizer", 2.0, "", json!(null)),
        ("equalizer", 0.0, "", json!(null)),
        ("frame-step", 0.0, "pause", json!(true)),
        ("frame-back-step", 0.0, "pause", json!(true)),
    ];
    let mut requests = Vec::new();
    for (name, value, property, expected) in scenarios {
        let effects = state.dispatch(Action::control(name, value).unwrap());
        let [Effect::Engine(command)] = effects.as_slice() else {
            panic!("Control did not reach engine");
        };
        requests
            .push(json!({"name":name,"command":command,"property":property,"expected":expected}));
    }
    requests.push(json!({
        "name": "stop-preserves-queue",
        "command": ["stop", "keep-playlist"],
        "property": "playlist-count",
        "expected": 1
    }));
    let requests_path = probe.directory.join("requests.json");
    let output_path = probe.directory.join("result.json");
    fs::write(&requests_path, serde_json::to_vec(&requests).unwrap()).unwrap();
    for name in ["one.srt", "two.srt"] {
        fs::write(
            probe.directory.join(name),
            "1\n00:00:00,000 --> 00:00:09,000\nSubtitle fixture\n",
        )
        .unwrap();
    }
    let log = fs::File::create(probe.directory.join("engine.log")).unwrap();
    probe.child = Some(
        Command::new(executable)
            .args([
                "--no-config",
                "--load-scripts=no",
                "--idle=yes",
                "--vo=null",
                "--ao=null",
                "--hwdec=no",
                "--length=8",
            ])
            .arg(format!(
                "--script={}",
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/controls.lua")
                    .display()
            ))
            .arg(format!(
                "--sub-file={}",
                probe.directory.join("one.srt").display()
            ))
            .arg(format!(
                "--sub-file={}",
                probe.directory.join("two.srt").display()
            ))
            .arg(fixture)
            .env("FOXIDE_CONTROL_REQUESTS", &requests_path)
            .env("FOXIDE_CONTROL_RESULTS", &output_path)
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = probe.child.as_mut().unwrap().try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let child = probe.child.as_mut().unwrap();
            let _ = child.kill();
            let _ = child.wait();
            let log = fs::read_to_string(probe.directory.join("engine.log")).unwrap_or_default();
            panic!("Native control probe timed out: {log}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let log = fs::read_to_string(probe.directory.join("engine.log")).unwrap();

    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(output_path).expect(&log)).unwrap();
    assert_eq!(
        result["completed"].as_u64(),
        Some(requests.len() as u64),
        "{result}\n{log}"
    );
    assert_eq!(result["errors"], json!([]), "{result}\n{log}");
    assert!(status.success(), "{result}\n{log}");
}

