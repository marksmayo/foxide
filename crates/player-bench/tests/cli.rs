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
            "0.7",
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
    assert!(
        fs::read_dir(&dir)
            .unwrap()
            .any(|entry| entry.unwrap().path().join("run.json").is_file())
    );
    fs::remove_dir_all(dir).unwrap();
}
