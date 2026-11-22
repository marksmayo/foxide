use player_services::process::{RunLimits, run_bounded, trusted_executable};
use std::{
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn helper_execution_preserves_arguments_and_enforces_failure_limits_and_cancellation() {
    let dir = Scratch(std::env::temp_dir().join(format!(
            "foxide-tool-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir(&dir.0).unwrap();
    let executable = dir.0.join(if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    });
    let compiled =
        std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tool.rs"))
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap();
    assert!(compiled.success());
    let cancel = AtomicBool::new(false);
    let limits = RunLimits {
        timeout: Duration::from_secs(15),
        stdout: 128,
        stderr: 512,
    };
    let literal = "media with spaces; $(ignored) & punctuation";
    let output = run_bounded(
        &executable,
        &["echo".into(), literal.into()],
        limits,
        &cancel,
    )
    .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), literal);
    let overflow = run_bounded(&executable, &["large".into()], limits, &cancel)
        .err()
        .unwrap();
    assert!(overflow.contains("limit"), "{overflow}");
    let failure = run_bounded(&executable, &["fail".into()], limits, &cancel)
        .err()
        .unwrap();
    assert!(failure.contains('7') && failure.contains("fixture failure"));
    assert!(trusted_executable(std::path::Path::new("relative-tool")).is_err());
    assert!(trusted_executable(&dir.0).is_err());
    assert!(
        run_bounded(
            &executable,
            &[],
            RunLimits {
                timeout: Duration::ZERO,
                ..limits
            },
            &cancel
        )
        .is_err()
    );
    let ready = dir.0.join("ready");
    let survived = dir.0.join("survived");
    let args: Vec<OsString> = vec![
        "sleep".into(),
        ready.as_os_str().into(),
        survived.as_os_str().into(),
    ];
    assert!(
        run_bounded(
            &executable,
            &args,
            RunLimits {
                timeout: Duration::from_millis(200),
                ..limits
            },
            &cancel
        )
        .err()
        .unwrap()
        .contains("timed out")
    );
    std::thread::sleep(Duration::from_millis(1700));
    assert!(
        !survived.exists(),
        "Timed out child must not continue writing"
    );
    let _ = std::fs::remove_file(&ready);
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let watched = ready.clone();
    let canceller = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !watched.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        flag.store(true, Ordering::Release);
    });
    let result = run_bounded(&executable, &args, limits, &cancelled);
    canceller.join().unwrap();
    assert!(result.err().unwrap().contains("cancelled"));
    std::thread::sleep(Duration::from_millis(1700));
    assert!(
        !survived.exists(),
        "Cancelled child must not continue writing"
    );
    assert!(run_bounded(&executable, &[], limits, &cancelled).is_err());
}
