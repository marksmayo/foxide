use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub struct RunLimits {
    pub timeout: Duration,
    pub stdout: usize,
    pub stderr: usize,
}
impl Default for RunLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(20),
            stdout: super::radio::RESPONSE_LIMIT,
            stderr: 8192,
        }
    }
}

pub fn trusted_executable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Choose an absolute executable path".into());
    }
    let path = path
        .canonicalize()
        .map_err(|e| format!("Cannot locate executable: {e}"))?;
    let metadata = path.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("Executable must be a file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("File is not executable".into());
        }
    }
    #[cfg(windows)]
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return Err("Choose a native .exe file".into());
    }
    Ok(path)
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct ToolOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Captures only this child, with bounded pipes and a deadline. No shell is involved.
pub fn run_bounded(
    executable: &Path,
    args: &[OsString],
    limits: RunLimits,
    cancel: &AtomicBool,
) -> Result<ToolOutput, String> {
    run_bounded_with_input(executable, args, limits, cancel, None)
}

pub fn run_bounded_with_input(
    executable: &Path,
    args: &[OsString],
    limits: RunLimits,
    cancel: &AtomicBool,
    input: Option<&[u8]>,
) -> Result<ToolOutput, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Operation cancelled".into());
    }
    if limits.timeout.is_zero() || limits.stdout > 64 * 1024 * 1024 || limits.stderr > 1024 * 1024 {
        return Err("Invalid process limits".into());
    }
    let executable = trusted_executable(executable)?;
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    } // CREATE_NO_WINDOW
    let mut child = OwnedChild(command.spawn().map_err(|e| e.to_string())?);
    if let Some(input) = input {
        let mut stdin = child.0.stdin.take().ok_or("Missing input pipe")?;
        stdin.write_all(input).map_err(|e| e.to_string())?;
    }
    let (sender, receiver) = mpsc::channel();
    let mut readers = Vec::new();
    for (stdout, pipe, limit) in [
        (
            true,
            Box::new(child.0.stdout.take().ok_or("Missing output pipe")?) as Box<dyn Read + Send>,
            limits.stdout,
        ),
        (
            false,
            Box::new(child.0.stderr.take().ok_or("Missing error pipe")?) as Box<dyn Read + Send>,
            limits.stderr,
        ),
    ] {
        let sender = sender.clone();
        readers.push(std::thread::spawn(move || {
            let mut data = Vec::new();
            let result = pipe
                .take(limit as u64 + 1)
                .read_to_end(&mut data)
                .map_err(|e| e.to_string())
                .and_then(|_| {
                    if data.len() > limit {
                        Err("Tool output exceeded its limit".into())
                    } else {
                        Ok(data)
                    }
                });
            let _ = sender.send((stdout, result));
        }));
    }
    drop(sender);
    let deadline = Instant::now() + limits.timeout;
    let (mut output, mut errors) = (None, None);
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("Operation cancelled".into());
        }
        if Instant::now() >= deadline {
            return Err("Operation timed out".into());
        }
        while let Ok((stdout, result)) = receiver.try_recv() {
            if stdout {
                output = Some(result?);
            } else {
                errors = Some(result?);
            }
        }
        if let Some(status) = child.0.try_wait().map_err(|e| e.to_string())?
            && let (Some(stdout), Some(stderr)) = (output.as_mut(), errors.as_mut())
        {
            for reader in readers {
                reader.join().map_err(|_| "Output reader failed")?;
            }
            let (stdout, stderr) = (std::mem::take(stdout), std::mem::take(stderr));
            if !status.success() {
                return Err(format!(
                    "Tool exited with {status}: {}",
                    String::from_utf8_lossy(&stderr)
                        .chars()
                        .take(500)
                        .collect::<String>()
                ));
            }
            return Ok(ToolOutput { stdout, stderr });
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

