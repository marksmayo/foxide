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
