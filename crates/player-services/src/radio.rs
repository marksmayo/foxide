use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

pub const RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

pub trait HttpClient: Send + Sync {
    fn get(&self, url: &Url, cancel: &AtomicBool) -> Result<Vec<u8>, String>;

    fn propfind(&self, _: &Url, _: &[u8], _: &AtomicBool) -> Result<Vec<u8>, String> {
        Err("This HTTP client does not support WebDAV browsing".into())
    }

    fn propfind_authenticated(
        &self,
        url: &Url,
        body: &[u8],
        credentials: Option<&crate::webdav::WebDavCredentials>,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        if credentials.is_some() {
            return Err("This HTTP client does not support WebDAV authentication".into());
        }
        self.propfind(url, body, cancel)
    }
}

pub fn public_http_url(text: &str) -> Result<Url, String> {
    let url = Url::parse(text).map_err(|_| "Invalid URL")?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Use an HTTP(S) URL without embedded credentials".into());
    }
    Ok(url)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub url: Url,
    pub country: String,
    pub codec: String,
    pub bitrate: u32,
}

#[derive(Debug, Clone)]
pub struct RadioDirectory {
    base: Url,
    discover: bool,
    mirror: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl RadioDirectory {
    pub fn public() -> Self {
        let mut directory =
            Self::new("https://de1.api.radio-browser.info").expect("Static provider origin");
        directory.discover = true;
        directory
    }

    pub fn new(base: &str) -> Result<Self, String> {
        let base = public_http_url(base)?;
        let loopback = match base.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if base.scheme() != "https" && !loopback {
            return Err(
                "Directory APIs require HTTPS (HTTP is allowed for local fixtures only)".into(),
            );
        }
        if base.path() != "/" || base.query().is_some() || base.fragment().is_some() {
            return Err("Enter a directory server origin without a path or query".into());
        }
        Ok(Self {
            base,
            discover: false,
            mirror: Default::default(),
        })
    }

    pub fn search(
        &self,
        client: &impl HttpClient,
        query: &str,
        cancel: &AtomicBool,
    ) -> Result<Vec<Station>, String> {
        if !self.discover {
            return self.search_direct(client, query, cancel);
        }
        let query = query.trim();
        if query.is_empty() || query.chars().count() > 128 {
            return Err("Enter 1–128 characters to search".into());
        }
        let mut attempted = std::collections::HashSet::new();
        let mut last_error = "No directory mirrors available".to_owned();
        if cancel.load(Ordering::Acquire) {
            return Err("Search cancelled".into());
        }
        let cached = self.mirror.lock().ok().and_then(|mirror| mirror.clone());
        if let Some(mirror) = cached {
            attempted.insert(mirror.clone());
            let directory = Self::new(&format!("https://{mirror}/"))?;
            match directory.search_direct(client, query, cancel) {
                Ok(stations) => return Ok(stations),
                Err(error) => {
                    if cancel.load(Ordering::Acquire) {
                        return Err("Search cancelled".into());
                    }
                    last_error = error;
                    if let Ok(mut cached) = self.mirror.lock() {
                        *cached = None;
                    }
                }
            }
        }
        for bootstrap in [
            "https://de1.api.radio-browser.info/json/servers",
            "https://nl1.api.radio-browser.info/json/servers",
        ] {
            if cancel.load(Ordering::Acquire) {
                return Err("Search cancelled".into());
            }
            let body = match client.get(
                &Url::parse(bootstrap).expect("Static discovery URL"),
                cancel,
            ) {
                Ok(body) if body.len() <= RESPONSE_LIMIT => body,
                Ok(_) => {
                    last_error = "Discovery response is too large".into();
                    continue;
                }
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&body) else {
                last_error = "Invalid discovery response".into();
                continue;
            };
            let Some(servers) = value.as_array().filter(|servers| servers.len() <= 64) else {
                last_error = "Invalid directory mirror list".into();
                continue;
            };
            let mirrors: std::collections::HashSet<_> = servers
                .iter()
                .filter_map(|server| {
                    let name = server["name"].as_str()?.to_ascii_lowercase();
                    let prefix = name.strip_suffix(".api.radio-browser.info")?;
                    if prefix.is_empty()
                        || prefix.len() > 32
                        || !prefix
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    {
                        return None;
                    }
                    Some(name)
                })
                .collect();
            for mirror in mirrors {
                if attempted.len() >= 3 {
                    break;
                }
                if !attempted.insert(mirror.clone()) {
                    continue;
                }
                let directory = Self::new(&format!("https://{mirror}/"))?;
                match directory.search_direct(client, query, cancel) {
                    Ok(stations) => {
                        if let Ok(mut cached) = self.mirror.lock() {
                            *cached = Some(mirror);
                        }
                        return Ok(stations);
                    }
                    Err(error) => last_error = error,
                }
            }
            if attempted.len() >= 3 {
                break;
            }
        }
        Err(format!("Radio Browser unavailable: {last_error}"))
    }

    fn search_direct(
        &self,
        client: &impl HttpClient,
        query: &str,
        cancel: &AtomicBool,
    ) -> Result<Vec<Station>, String> {
        let query = query.trim();
        if query.is_empty() || query.chars().count() > 128 {
            return Err("Enter 1–128 characters to search".into());
        }
        if cancel.load(Ordering::Acquire) {
            return Err("Search cancelled".into());
        }
        let mut url = self
            .base
            .join("json/stations/search")
            .map_err(|e| e.to_string())?;
        url.query_pairs_mut().extend_pairs([
            ("name", query),
            ("limit", "50"),
            ("hidebroken", "true"),
            ("order", "votes"),
            ("reverse", "true"),
        ]);
        let body = client.get(&url, cancel)?;
        if cancel.load(Ordering::Acquire) {
            return Err("Search cancelled".into());
        }
        if body.len() > RESPONSE_LIMIT {
            return Err("Directory response is too large".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| "Directory returned invalid JSON")?;
        let stations = value
            .as_array()
            .ok_or("Directory response must be a station list")?;
        if stations.len() > 1000 {
            return Err("Directory returned too many stations".into());
        }
        Ok(stations
            .iter()
            .filter_map(|station| {
                if station["lastcheckok"].as_u64() != Some(1) {
                    return None;
                }
                let name = station["name"].as_str()?.trim();
                if name.is_empty() || name.chars().count() > 512 {
                    return None;
                }
                let id = station["stationuuid"].as_str()?;
                if id.is_empty()
                    || id.len() > 128
                    || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                {
                    return None;
                }
                let url = public_http_url(station["url_resolved"].as_str()?).ok()?;
                Some(Station {
                    id: id.into(),
                    name: name
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect(),
                    url,
                    country: station["country"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(80)
                        .collect(),
                    codec: station["codec"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(32)
                        .collect(),
                    bitrate: station["bitrate"].as_u64().unwrap_or(0).min(10000) as u32,
                })
            })
            .take(50)
            .collect())
    }

    pub fn click(
        &self,
        client: &impl HttpClient,
        station_id: &str,
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        if station_id.is_empty()
            || station_id.len() > 128
            || !station_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("Invalid station identifier".into());
        }
        let url = self
            .base
            .join(&format!("json/url/{station_id}"))
            .map_err(|e| e.to_string())?;
        client.get(&url, cancel).map(|_| ())
    }
}
