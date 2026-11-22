fn main() {
    if let Ok(path) = std::env::var("FOXIDE_TEST_ENGINE_PID_LOG") {
        std::fs::write(path, std::process::id().to_string()).unwrap();
    }
    if let Ok(path) = std::env::var("FOXIDE_TEST_ENGINE_LOG") {
        std::fs::write(path, std::env::args().collect::<Vec<_>>().join("\n")).unwrap();
    }
    if std::env::var("FOXIDE_TEST_ENGINE_FAIL").is_ok() { std::process::exit(7); }
}
