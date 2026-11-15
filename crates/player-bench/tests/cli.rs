use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp(name: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
    fs::create_dir_all(&base).unwrap();
    let path = base.canonicalize().unwrap().join(format!(
        "foxide test {name} {} {}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn cold_reboot_requires_attestation_and_one_run() {
    let output = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .args([
            "run",
            "--player",
            "mpv",
            "--media",
            "unused",
            "--cache",
            "cold-reboot",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cold-reboot requires"));
}

#[test]
fn samples_are_recorded_without_inventing_video_or_power_measurements() {
    let dir = temp("sampling");
    let media = dir.join("fixture with spaces.txt");
    fs::write(&media, "A non-media process-control fixture").unwrap();
    let python = std::env::var("FOXIDE_PYTHON").unwrap_or_else(|_| "python".into());
    let output = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .args([
            "run",
            "--player",
            "external",
            "--exe",
            &python,
            "--arg=-c",
            "--arg=import time; time.sleep(10)",
            "--duration",
            "0.7",
            "--warmup",
            "0.1",
            "--sample-ms",
            "100",
            "--repeat",
            "1",
            "--media",
        ])
        .arg(&media)
        .arg("--output")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let runs: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path().join("run.json");
            path.exists().then_some(path)
        })
        .collect();
    assert_eq!(runs.len(), 1);
    let run: serde_json::Value = serde_json::from_slice(&fs::read(&runs[0]).unwrap()).unwrap();
    assert_eq!(run["status"], "process-sampled-playback-unverified");
    assert!(run["ttff_display_ms"].is_null());
    assert!(run["startup_restart_observed_ms"].is_null());
    assert!(run["package_power_watts"].is_null());
    assert!(run["rss_peak_bytes"].as_u64().unwrap() > 0);
    assert!(run["samples"].as_array().unwrap().len() >= 2);
    assert_eq!(run["terminated_by_harness"], true);
    let report = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .arg("report")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(report.status.success());
    assert!(String::from_utf8_lossy(&report.stdout).contains("external / clean-defaults"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn premature_exit_is_saved_and_returns_failure() {
    let dir = temp("early exit");
    let media = dir.join("fixture.txt");
    fs::write(&media, "test").unwrap();
    let python = std::env::var("FOXIDE_PYTHON").unwrap_or_else(|_| "python".into());
    let output = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .args([
            "run",
            "--player",
            "external",
            "--exe",
            &python,
            "--arg=-c",
            "--arg=raise SystemExit(7)",
            "--duration",
            "3",
            "--warmup",
            "0.1",
            "--repeat",
            "1",
            "--media",
        ])
        .arg(&media)
        .arg("--output")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let saved = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("run.json"))
        .find(|path| path.is_file())
        .expect("early exits must preserve their run record");
    let record: serde_json::Value = serde_json::from_slice(&fs::read(saved).unwrap()).unwrap();
    assert_eq!(record["status"], "incomplete-player-exited");
    assert!(record["exit_status"].as_str().unwrap().contains('7'));
    assert_eq!(record["terminated_by_harness"], false);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn diagnostic_and_invalid_sampling_commands_fail_before_playback() {
    let doctor = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .arg("doctor")
        .output()
        .unwrap();
    assert!(doctor.status.success());
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("Display TTFF"));
    for args in [
        vec!["--duration", "NaN"],
        vec!["--duration", "86401"],
        vec!["--warmup", "-1"],
        vec!["--duration", "1", "--warmup", "2"],
        vec!["--sample-ms", "10"],
        vec!["--repeat", "0"],
        vec!["--tuned"],
        vec!["--seeks"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_player-bench"))
            .args(["run", "--player", "vlc", "--media", "unused"])
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
}

#[test]
fn warm_priming_and_engine_events_never_become_display_measurements() {
    let dir = temp("engine protocol");
    let executable = dir.join(if cfg!(windows) {
        "protocol-player.exe"
    } else {
        "protocol-player"
    });
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let built = Command::new(compiler)
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/protocol_player.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let media = dir.join("fixture.txt");
    fs::write(&media, "Protocol fixture, not media").unwrap();
    // Windows process creation under instrumentation can exceed 700ms.
    // This is a protocol-contract check, not a startup-performance assertion.
    let output = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .args(["run", "--player", "mpv", "--exe"])
        .arg(executable)
        .args([
            "--tuned",
            "--seeks",
            "--cache",
            "warm",
            "--repeat",
            "1",
            "--duration",
            "3",
            "--warmup",
            "0.1",
            "--sample-ms",
            "100",
            "--media",
        ])
        .arg(&media)
        .arg("--output")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut measured = 0;
    let mut primed = 0;
    for path in fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()) {
        if path.join("priming.json").is_file() {
            primed += 1;
        }
        if path.join("run.json").is_file() {
            measured += 1;
            let run: serde_json::Value =
                serde_json::from_slice(&fs::read(path.join("run.json")).unwrap()).unwrap();
            assert_eq!(run["status"], "internal-restart-observed");
            assert_eq!(run["profile"], "hwdec-auto-safe-seeks");
            assert!(run["startup_restart_observed_ms"].is_number());
            assert!(run["ttff_display_ms"].is_null());
            assert!(run["package_power_watts"].is_null());
        }
    }
    assert_eq!((primed, measured), (1, 1));
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-tmp")
        .canonicalize()
        .unwrap();
    assert!(dir.starts_with(base));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn observing_an_existing_process_never_terminates_it() {
    let dir = temp("observe");
    let media = dir.join("fixture.txt");
    fs::write(&media, "Process observation fixture").unwrap();
    let python = std::env::var("FOXIDE_PYTHON").unwrap_or_else(|_| "python".into());
    let mut child = Command::new(python)
        .args(["-c", "import time;time.sleep(30)"])
        .spawn()
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_player-bench"))
        .args([
            "observe",
            "--pid",
            &child.id().to_string(),
            "--label",
            "owned-test-process",
            "--duration",
            "0.7",
            "--warmup",
            "0.1",
            "--sample-ms",
            "100",
            "--media",
        ])
        .arg(media)
        .arg("--output")
        .arg(&dir)
        .output()
        .unwrap();
    let still_running = child.try_wait().unwrap().is_none();
    let _ = child.kill();
    child.wait().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(still_running);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-tmp")
        .canonicalize()
        .unwrap();
    assert!(dir.starts_with(base));
    fs::remove_dir_all(dir).unwrap();
}
