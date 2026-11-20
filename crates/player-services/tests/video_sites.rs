use player_services::VideoSiteTool;

#[test]
fn video_site_helper_is_explicit_and_never_adds_a_path_search_fallback() {
    let executable = std::env::current_exe().unwrap();
    let tool = VideoSiteTool::new(&executable).unwrap();
    assert_eq!(
        tool.path().canonicalize().unwrap(),
        executable.canonicalize().unwrap()
    );
    assert_eq!(
        tool.script_option(),
        format!("ytdl_hook-ytdl_path={}", tool.path().to_str().unwrap())
    );
    assert!(VideoSiteTool::new(std::path::Path::new("yt-dlp")).is_err());
    assert!(VideoSiteTool::new(&std::env::temp_dir()).is_err());
}

#[test]
fn an_existing_executable_cannot_be_interpreted_as_a_helper_path_list() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../results/local/helper-validation");
    std::fs::create_dir_all(&root).unwrap();
    let separator = if cfg!(windows) { ';' } else { ':' };
    let file = root.join(format!(
        "unsafe{separator}{}-{}.exe",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let outcome = VideoSiteTool::new(&file);
    std::fs::remove_file(file).unwrap();
    assert!(outcome.err().unwrap().contains("path-list"));
}
