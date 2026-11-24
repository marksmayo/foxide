use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 512 * 1024;

#[derive(Clone, Copy)]
#[allow(dead_code)] // Tests exercise each platform resolver on the current host.
enum LogPlatform {
    Windows,
    MacOs,
    Linux,
}

#[cfg(windows)]
fn current_log_platform() -> LogPlatform {
    LogPlatform::Windows
}

#[cfg(target_os = "macos")]
fn current_log_platform() -> LogPlatform {
    LogPlatform::MacOs
}

#[cfg(not(any(windows, target_os = "macos")))]
fn current_log_platform() -> LogPlatform {
    LogPlatform::Linux
}

pub(super) fn log_path() -> Result<PathBuf, String> {
    log_path_for(
        current_log_platform(),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

fn log_path_for(
    platform: LogPlatform,
    local_app_data: Option<PathBuf>,
    state_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, String> {
    let base = match platform {
        LogPlatform::Windows => local_app_data
            .ok_or("Local application data folder is unavailable")?
            .join("Foxide")
            .join("logs"),
        LogPlatform::MacOs => home
            .ok_or("Home folder is unavailable")?
            .join("Library")
            .join("Logs")
            .join("Foxide"),
        LogPlatform::Linux => state_home
            .or_else(|| home.map(|home| home.join(".local").join("state")))
            .ok_or("User state folder is unavailable")?
            .join("foxide")
            .join("logs"),
    };
    Ok(base.join("foxide.log"))
}

pub(super) fn record(event: &str) {
    if let Ok(path) = log_path() {
        let _ = append_at(&path, event);
    }
}

fn append_at(path: &std::path::Path, event: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("Log path has no parent folder")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES)
    {
        let previous = path.with_extension("log.1");
        let _ = fs::remove_file(&previous);
        fs::rename(path, previous).map_err(|error| error.to_string())?;
    }
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    writeln!(file, "{elapsed} {} {event}", env!("CARGO_PKG_VERSION"))
        .map_err(|error| error.to_string())
}

pub(super) fn open_log(
    resolve_path: impl FnOnce() -> Result<PathBuf, String>,
    open: impl FnOnce(&std::path::Path) -> Result<(), String>,
) -> Result<(), String> {
    let path = resolve_path()?;
    open_log_at(&path, open)
}

fn open_log_at(
    path: &std::path::Path,
    open: impl FnOnce(&std::path::Path) -> Result<(), String>,
) -> Result<(), String> {
    if !path.exists() {
        append_at(path, "Diagnostic log opened; no earlier events recorded")?;
    }
    open(path)
}

pub(super) fn open_system_log(
    path: &std::path::Path,
    launch: impl FnOnce(&mut Command) -> Result<(), String>,
) -> Result<(), String> {
    let mut command = system_log_command(path);
    launch(&mut command)
}

fn system_log_command(path: &std::path::Path) -> Command {
    system_log_command_for(current_log_platform(), path)
}

fn system_log_command_for(platform: LogPlatform, path: &std::path::Path) -> Command {
    let mut command = match platform {
        LogPlatform::Windows => {
            let mut command = Command::new("explorer.exe");
            command.arg(path);
            command
        }
        LogPlatform::MacOs => {
            let mut command = Command::new("open");
            command.arg(path);
            command
        }
        LogPlatform::Linux => {
            let mut command = Command::new("xdg-open");
            command.arg(path);
            command
        }
    };
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_is_append_only_until_the_size_limit_then_rotates() {
        let temp = std::env::temp_dir().join(format!("foxide-diagnostics-{}", std::process::id()));
        let path = temp.join("foxide.log");
        let _ = fs::remove_dir_all(&temp);
        append_at(&path, "first event").unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("first event"));
        fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();
        append_at(&path, "after rotation").unwrap();
        assert!(path.with_extension("log.1").exists());
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("after rotation")
        );
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn opening_a_missing_log_creates_it_before_calling_the_viewer() {
        let temp = std::env::temp_dir().join(format!("foxide-log-open-{}", std::process::id()));
        let path = temp.join("foxide.log");
        let _ = fs::remove_dir_all(&temp);
        let mut viewer_called = false;
        open_log(
            || Ok(path.clone()),
            |opened| {
                viewer_called = true;
                assert_eq!(opened, path);
                assert!(
                    fs::read_to_string(opened)
                        .unwrap()
                        .contains("no earlier events recorded")
                );
                Ok(())
            },
        )
        .unwrap();
        assert!(viewer_called);
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn failed_path_resolution_does_not_call_the_viewer() {
        let mut viewer_called = false;
        let result = open_log(
            || Err("log path unavailable".into()),
            |_| {
                viewer_called = true;
                Ok(())
            },
        );
        assert_eq!(result.unwrap_err(), "log path unavailable");
        assert!(!viewer_called);
    }

    #[test]
    fn system_viewer_command_points_at_the_log_file() {
        let path = PathBuf::from("local-log-test/foxide.log");
        let mut viewer_called = false;
        open_system_log(&path, |command| {
            viewer_called = true;
            assert_eq!(command.get_args().last(), Some(path.as_os_str()));
            Ok(())
        })
        .unwrap();
        assert!(viewer_called);
    }

    #[test]
    fn log_path_uses_the_platform_log_directory_and_fallbacks() {
        let path = |platform: LogPlatform,
                    local: Option<&str>,
                    state: Option<&str>,
                    home: Option<&str>| {
            log_path_for(
                platform,
                local.map(PathBuf::from),
                state.map(PathBuf::from),
                home.map(PathBuf::from),
            )
        };
        assert_eq!(
            path(
                LogPlatform::Windows,
                Some("C:/Users/test/AppData/Local"),
                None,
                None
            )
            .unwrap(),
            PathBuf::from("C:/Users/test/AppData/Local/Foxide/logs/foxide.log")
        );
        assert_eq!(
            path(LogPlatform::MacOs, None, None, Some("/Users/test")).unwrap(),
            PathBuf::from("/Users/test/Library/Logs/Foxide/foxide.log")
        );
        assert_eq!(
            path(LogPlatform::Linux, None, Some("/state"), Some("/home/test")).unwrap(),
            PathBuf::from("/state/foxide/logs/foxide.log")
        );
        assert_eq!(
            path(LogPlatform::Linux, None, None, Some("/home/test")).unwrap(),
            PathBuf::from("/home/test/.local/state/foxide/logs/foxide.log")
        );
        assert_eq!(
            path(LogPlatform::Windows, None, None, None).unwrap_err(),
            "Local application data folder is unavailable"
        );
        assert_eq!(
            path(LogPlatform::MacOs, None, None, None).unwrap_err(),
            "Home folder is unavailable"
        );
        assert_eq!(
            path(LogPlatform::Linux, None, None, None).unwrap_err(),
            "User state folder is unavailable"
        );
    }

    #[test]
    fn viewer_command_is_selected_for_each_platform() {
        let path = PathBuf::from("foxide.log");
        for (platform, program) in [
            (LogPlatform::Windows, "explorer.exe"),
            (LogPlatform::MacOs, "open"),
            (LogPlatform::Linux, "xdg-open"),
        ] {
            let command = system_log_command_for(platform, &path);
            assert_eq!(command.get_program(), program);
            assert_eq!(command.get_args().last(), Some(path.as_os_str()));
        }
    }

    #[test]
    fn append_reports_a_log_path_without_a_parent() {
        assert_eq!(
            append_at(std::path::Path::new(""), "event").unwrap_err(),
            "Log path has no parent folder"
        );
    }
}
