use crate::{
    HttpClient,
    process::{RunLimits, run_bounded, trusted_executable},
    public_http_url,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use url::Url;

#[derive(Clone)]
pub struct CurlClient {
    executable: PathBuf,
}
impl CurlClient {
    pub fn new(executable: &Path) -> Result<Self, String> {
        Ok(Self {
            executable: trusted_executable(executable)?,
        })
    }
    pub fn system() -> Result<Self, String> {
        #[cfg(windows)]
        let paths: Vec<PathBuf> = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .map(|p| vec![p.join("System32/curl.exe")])
            .unwrap_or_default();
        #[cfg(unix)]
        let paths: Vec<PathBuf> = ["/usr/bin/curl", "/bin/curl"]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        paths
            .into_iter()
            .find_map(|path| Self::new(&path).ok())
            .ok_or("System curl was not found; install curl to use online services".into())
    }

    /// Download one HTTPS resource to a private temporary path, enforcing a byte cap.
    pub fn download_file(
        &self,
        url: &Url,
        destination: &Path,
        max_bytes: u64,
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let loopback = match url.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
            || !url.username().is_empty()
            || url.password().is_some()
            || max_bytes == 0
            || max_bytes > 512 * 1024 * 1024
        {
            return Err("Update downloads require HTTPS and a valid size limit".into());
        }
        let parent = destination
            .parent()
            .ok_or("Download destination has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let mut temporary = destination.as_os_str().to_os_string();
        temporary.push(format!(".{}.part", std::process::id()));
        let temporary = PathBuf::from(temporary);
        let _ = std::fs::remove_file(&temporary);
        let mut args: Vec<std::ffi::OsString> = [
            "--disable",
            "--silent",
            "--show-error",
            "--fail",
            "--globoff",
            "--location",
            "--max-redirs",
            "3",
            "--proto",
            if loopback { "=http,https" } else { "=https" },
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "--connect-timeout",
            "10",
            "--max-time",
            "900",
            "--max-filesize",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        args.push(max_bytes.to_string().into());
        if loopback {
            args.extend(["--noproxy".into(), "*".into()]);
        }
        args.extend([
            "--user-agent".into(),
            "Foxide/0.0.1".into(),
            "--output".into(),
            temporary.as_os_str().to_owned(),
            "--url".into(),
            url.as_str().into(),
        ]);
        let result = run_bounded(
            &self.executable,
            &args,
            RunLimits {
                timeout: std::time::Duration::from_secs(900),
                stdout: 1024,
                stderr: 8192,
            },
            cancel,
        );
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        let size = match std::fs::metadata(&temporary) {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                return Err(error.to_string());
            }
        };
        if size == 0 || size > max_bytes {
            let _ = std::fs::remove_file(&temporary);
            return Err("Downloaded file size is invalid".into());
        }
        if destination.exists() {
            std::fs::remove_file(destination).map_err(|error| error.to_string())?;
        }
        std::fs::rename(&temporary, destination).map_err(|error| {
            let _ = std::fs::remove_file(&temporary);
            error.to_string()
        })
    }
}
impl HttpClient for CurlClient {
    fn get(&self, url: &Url, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
        public_http_url(url.as_str())?;
        let mut args: Vec<std::ffi::OsString> = [
            "--disable",
            "--silent",
            "--show-error",
            "--fail",
            "--globoff",
            "--location",
            "--max-redirs",
            "3",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
            "--max-filesize",
            "2097152",
            "--user-agent",
            "Foxide/0.0.1",
            "--output",
            "-",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let loopback = match url.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if loopback {
            args.extend(["--noproxy".into(), "*".into()]);
        }
        args.extend(["--url".into(), url.as_str().into()]);
        run_bounded(&self.executable, &args, RunLimits::default(), cancel)
            .map(|result| result.stdout)
    }
}
