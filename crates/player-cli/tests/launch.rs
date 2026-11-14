use std::process::Command;
use std::{fs, path::PathBuf, sync::OnceLock};

fn scratch() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
    fs::create_dir_all(&root).unwrap();
    let path = root.join(format!(
        "launcher-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&path).unwrap();
    path
}

fn engine() -> &'static PathBuf {
    static ENGINE: OnceLock<PathBuf> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let output = scratch().join(if cfg!(windows) {
            "engine.exe"
        } else {
            "engine"
        });
        let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let status = Command::new(compiler)
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/engine.rs"))
            .arg("-o")
            .arg(&output)
            .status()
            .unwrap();
        assert!(status.success());
        output
    })
}

#[test]
fn user_can_review_a_network_playlist_with_playback_controls() {
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .args([
            "--print-command",
            "--speed",
            "2",
            "--audio-track",
            "2",
            "--subtitle-delay",
            "-0.5",
            "--minimal",
            "--on-top",
            "--tone-map",
            "https://example.org/movie.m3u8",
            "rtsp://example.org/live",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "--speed=2",
        "--aid=2",
        "--sub-delay=-0.5",
        "--border=no",
        "--ontop=yes",
        "--target-trc=bt.1886",
        "https://example.org/movie.m3u8",
        "rtsp://example.org/live",
    ] {
        assert!(plan.contains(expected), "missing {expected}: {plan}");
    }
}

#[test]
fn local_folder_queue_starts_at_selected_episode_and_loads_subtitles() {
    let dir = scratch();
    for name in ["episode10.mkv", "episode2.mp4", "episode1.mp4", "subs.ass"] {
        fs::write(dir.join(name), "test fixture").unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .args(["--print-command", "--autoqueue", "--subtitle"])
        .arg(dir.join("subs.ass"))
        .args(["--subtitle-track", "1", "--resume-dir"])
        .arg(dir.join("state"))
        .arg(dir.join("episode2.mp4"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(args.iter().any(|a| a == "--playlist-start=1"));
    assert!(args.iter().any(|a| a == "--sid=1"));
    let start = args.iter().position(|a| a == "--").unwrap();
    assert_eq!(
        args[start + 1..]
            .iter()
            .map(|p| PathBuf::from(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned())
            .collect::<Vec<_>>(),
        ["episode1.mp4", "episode2.mp4", "episode10.mkv"]
    );
    let folder = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .arg("--print-command")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(folder.status.success());
    let normal = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .args([
            "--print-command",
            "--chapter",
            "0",
            "--ab-start",
            "1",
            "--ab-end",
            "3",
            "--loop-playlist",
            "--deinterlace",
            "--headless",
            "--paused",
            "--benchmark-script",
        ])
        .arg(dir.join("subs.ass"))
        .arg(dir.join("episode1.mp4"))
        .output()
        .unwrap();
    assert!(normal.status.success());
    assert!(String::from_utf8_lossy(&normal.stdout).contains("--pause=yes"));
}

#[test]
fn engine_is_started_with_safe_argument_boundaries_and_errors_propagate() {
    let dir = scratch();
    let media = dir.join("media with spaces.mp4");
    let log = dir.join("engine.log");
    fs::write(&media, "Process-only fixture").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .arg("--mpv")
        .arg(engine())
        .arg(&media)
        .env("FOXIDE_TEST_ENGINE_LOG", &log)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let command = fs::read_to_string(log).unwrap();
    assert!(command.contains("--hwdec=auto-safe\n"));
    assert!(command.contains("\n--\n"));
    assert!(command.contains("media with spaces.mp4"));
    let failed = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .arg("--mpv")
        .arg(engine())
        .arg(&media)
        .env("FOXIDE_TEST_ENGINE_FAIL", "1")
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("mpv exited"));
    let missing = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .arg("--mpv")
        .arg(dir.join("does-not-exist"))
        .arg(&media)
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Install mpv"));
}

#[test]
fn discovery_respects_local_installation_and_explicit_overrides() {
    let dir = scratch();
    fs::create_dir_all(dir.join("tools/mpv")).unwrap();
    let local = dir
        .join("tools/mpv")
        .join(if cfg!(windows) { "mpv.exe" } else { "mpv" });
    fs::write(&local, "Discovery-only fixture").unwrap();
    let media = dir.join("fixture.mp4");
    fs::write(&media, []).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .current_dir(&dir)
        .env_remove("FOXIDE_MPV")
        .arg("--print-command")
        .arg(&media)
        .output()
        .unwrap();
    assert!(output.status.success());
    let plan: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        PathBuf::from(&plan[0]).canonicalize().unwrap(),
        local.canonicalize().unwrap()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .env("FOXIDE_MPV", "preferred-engine")
        .arg("--print-command")
        .arg(&media)
        .output()
        .unwrap();
    assert!(output.status.success());
    let plan: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan[0], "preferred-engine");
}

#[test]
fn invalid_media_and_control_combinations_fail_before_engine_start() {
    let dir = scratch();
    let path = dir.join("fixture.mp4");
    fs::write(&path, []).unwrap();
    for extra in [
        vec!["--speed", "0"],
        vec!["--autoqueue", "https://example.org/a"],
        vec!["--subtitle", "missing.srt"],
        vec!["--ab-start", "10", "--ab-end", "2"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
            .arg("--print-command")
            .args(extra)
            .arg(&path)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
    let empty = dir.join("empty");
    fs::create_dir(&empty).unwrap();
    for input in [empty, dir.join("missing.mp4")] {
        let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
            .arg("--print-command")
            .arg(input)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}
