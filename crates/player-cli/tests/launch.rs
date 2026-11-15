use std::process::Command;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}
impl AsRef<std::ffi::OsStr> for Scratch {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_os_str()
    }
}
impl AsRef<std::path::Path> for Scratch {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        // Created uniquely below the test-owned root, never borrowed from a user.
        fs::remove_dir_all(&self.0).expect("Remove launcher fixture");
    }
}

fn scratch() -> Scratch {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-tmp");
    fs::create_dir_all(&root).unwrap();
    let path = root.join(format!(
        "launcher-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    Scratch(path)
}

fn engine(dir: &std::path::Path) -> PathBuf {
    let output = dir.join(if cfg!(windows) {
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
    let engine = engine(&dir);
    let media = dir.join("media with spaces.mp4");
    let log = dir.join("engine.log");
    fs::write(&media, "Process-only fixture").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_foxide"))
        .arg("--mpv")
        .arg(&engine)
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
        .arg(&engine)
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
fn discovery_ignores_dependencies_planted_in_the_working_directory() {
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
    assert_ne!(
        PathBuf::from(&plan[0]).canonicalize().ok(),
        Some(local.canonicalize().unwrap())
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

#[test]
fn absent_engine_never_falls_back_to_an_implicit_working_directory_search() {
    let dir = scratch();
    let app = dir.join("app");
    let media = dir.join("media");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(media.join("tools/mpv")).unwrap();
    let executable = app.join(if cfg!(windows) {
        "foxide.exe"
    } else {
        "foxide"
    });
    fs::copy(env!("CARGO_BIN_EXE_foxide"), &executable).unwrap();
    let name = if cfg!(windows) { "mpv.exe" } else { "mpv" };
    fs::write(media.join(name), "planted bare executable").unwrap();
    fs::write(media.join("tools/mpv").join(name), "planted tool").unwrap();
    let output = Command::new(&executable)
        .current_dir(&media)
        .env_remove("FOXIDE_MPV")
        .env("PATH", "")
        .args(["--print-command", "https://example.org/fixture.mp4"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    let planned = PathBuf::from(&plan[0]);
    assert!(planned.is_absolute());
    assert_eq!(
        planned.parent().unwrap().canonicalize().unwrap(),
        app.canonicalize().unwrap()
    );
    assert_eq!(planned.file_name(), Some(std::ffi::OsStr::new(name)));
}
