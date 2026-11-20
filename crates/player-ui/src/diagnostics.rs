use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 512 * 1024;

pub(super) fn log_path() -> Result<PathBuf, String> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or("Local application data folder is unavailable")?
            .join("Foxide")
            .join("logs")
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Home folder is unavailable")?
            .join("Library")
            .join("Logs")
            .join("Foxide")
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
            .ok_or("User state folder is unavailable")?
            .join("foxide")
            .join("logs")
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

pub(super) fn open_log() -> Result<(), String> {
    let path = log_path()?;
    if !path.exists() {
        record("Foxide started; no diagnostic events recorded yet");
    }
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(&path);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&path);
        command
    };
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(&path);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
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
}
