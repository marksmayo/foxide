use crate::{HttpClient, public_http_url};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use url::Url;

const SEARCH_RESPONSE_LIMIT: usize = 2 * 1024 * 1024;
const SUBTITLE_RESPONSE_LIMIT: usize = 5 * 1024 * 1024;
const RESULT_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleResult {
    pub file_id: String,
    pub file_name: String,
    pub title: String,
    pub release: String,
    pub language: String,
    pub download_count: u64,
}

#[derive(Debug, Clone)]
pub struct SubtitleProvider {
    base: Url,
}

impl SubtitleProvider {
    pub fn opensubtitles() -> Self {
        Self::new("https://api.opensubtitles.com/api/v1/").expect("Static API origin")
    }

    /// An origin override is provided for deterministic local protocol tests.
    pub fn new(origin: &str) -> Result<Self, String> {
        let base = public_http_url(origin)?;
        let local = match base.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if base.scheme() != "https" && !local {
            return Err("Subtitle services require HTTPS".into());
        }
        if base.query().is_some() || base.fragment().is_some() {
            return Err("Enter a subtitle API origin without a query or fragment".into());
        }
        Ok(Self { base })
    }

    pub fn search(
        &self,
        client: &impl HttpClient,
        api_key: &str,
        query: &str,
        language: &str,
        cancel: &AtomicBool,
    ) -> Result<Vec<SubtitleResult>, String> {
        validate_credentials(api_key)?;
        let query = query.trim();
        if query.is_empty() || query.chars().count() > 128 {
            return Err("Enter 1–128 characters to search".into());
        }
        if !valid_language(language) {
            return Err("Enter a two or three letter language code (for example, en)".into());
        }
        check_cancelled(cancel)?;

        let mut url = self
            .base
            .join("subtitles")
            .map_err(|_| "Invalid subtitle API URL")?;
        url.query_pairs_mut().extend_pairs([
            ("query", query),
            ("languages", language),
            ("order_by", "download_count"),
            ("order_direction", "desc"),
            ("limit", "20"),
        ]);
        let headers = [("Api-Key", api_key), ("User-Agent", "Foxide v0.0.1")];
        let body = client
            .get_with_headers(&url, &headers, cancel)
            .map_err(provider_error)?;
        check_cancelled(cancel)?;
        if body.len() > SEARCH_RESPONSE_LIMIT {
            return Err("Subtitle search response is too large".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| "Subtitle provider returned invalid JSON")?;
        let rows = value["data"]
            .as_array()
            .ok_or("Subtitle provider response is missing results")?;
        Ok(rows
            .iter()
            .filter_map(parse_result)
            .take(RESULT_LIMIT)
            .collect())
    }

    pub fn download_to(
        &self,
        client: &impl HttpClient,
        api_key: &str,
        subtitle: &SubtitleResult,
        directory: &Path,
        cancel: &AtomicBool,
    ) -> Result<PathBuf, String> {
        validate_credentials(api_key)?;
        if subtitle.file_id.is_empty()
            || subtitle.file_id.len() > 32
            || !subtitle.file_id.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("Invalid subtitle file identifier".into());
        }
        let extension = safe_extension(&subtitle.file_name).ok_or("Unsupported subtitle format")?;
        check_cancelled(cancel)?;
        let url = self
            .base
            .join("download")
            .map_err(|_| "Invalid subtitle API URL")?;
        let headers = [
            ("Api-Key", api_key),
            ("User-Agent", "Foxide v0.0.1"),
            ("Content-Type", "application/json"),
        ];
        let file_id = subtitle
            .file_id
            .parse::<u64>()
            .map_err(|_| "Invalid subtitle file identifier")?;
        let request = serde_json::json!({ "file_id": file_id });
        let response = client
            .post_json(&url, &headers, request.to_string().as_bytes(), cancel)
            .map_err(provider_error)?;
        check_cancelled(cancel)?;
        if response.len() > 64 * 1024 {
            return Err("Subtitle download response is too large".into());
        }
        let value: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|_| "Subtitle provider returned an invalid download response")?;
        let download_url = value["link"]
            .as_str()
            .ok_or("Subtitle provider returned no download link")?;
        let download_url = public_http_url(download_url)?;
        if download_url.scheme() != "https" {
            return Err("Subtitle download links must use HTTPS".into());
        }
        let bytes = client.get(&download_url, cancel).map_err(provider_error)?;
        check_cancelled(cancel)?;
        if bytes.is_empty() || bytes.len() > SUBTITLE_RESPONSE_LIMIT {
            return Err("Downloaded subtitle size is invalid".into());
        }
        validate_subtitle(&bytes, extension)?;

        let file_name = safe_file_name(&subtitle.file_name, extension)?;
        std::fs::create_dir_all(directory).map_err(|_| "Could not create the subtitle folder")?;
        let destination = directory.join(file_name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| -> String {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    String::from(
                        "A subtitle with that name already exists; remove or rename it first",
                    )
                } else {
                    String::from("Could not save the downloaded subtitle")
                }
            })?;
        use std::io::Write;
        if let Err(error) = file.write_all(&bytes) {
            let _ = std::fs::remove_file(&destination);
            return Err(format!("Could not save the downloaded subtitle: {error}"));
        }
        Ok(destination)
    }
}

fn validate_credentials(api_key: &str) -> Result<(), String> {
    if api_key.is_empty()
        || api_key.len() > 256
        || !api_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("Enter a valid OpenSubtitles application API key".into());
    }
    Ok(())
}

fn valid_language(language: &str) -> bool {
    matches!(language.len(), 2 | 3) && language.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn check_cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Subtitle operation cancelled".into())
    } else {
        Ok(())
    }
}

fn provider_error(error: String) -> String {
    if error.contains("429") {
        "OpenSubtitles rate limit reached. Wait before searching or downloading again".into()
    } else if error.contains("401") || error.contains("403") {
        "OpenSubtitles rejected the API key. Check the application key and account access".into()
    } else if error.contains("timed out") {
        "OpenSubtitles request timed out. Check the connection and try again".into()
    } else {
        format!("OpenSubtitles unavailable: {error}")
    }
}

fn parse_result(row: &serde_json::Value) -> Option<SubtitleResult> {
    let attributes = &row["attributes"];
    let file = attributes["files"].as_array()?.first()?;
    let file_id = file["file_id"].as_u64()?.to_string();
    let file_name = file["file_name"].as_str()?;
    let extension = safe_extension(file_name)?;
    Some(SubtitleResult {
        file_id,
        file_name: safe_file_name(file_name, extension).ok()?,
        title: bounded_text(
            attributes["feature_details"]["title"]
                .as_str()
                .unwrap_or("Subtitle"),
            160,
        ),
        release: bounded_text(attributes["release"].as_str().unwrap_or(""), 180),
        language: bounded_text(attributes["language"].as_str().unwrap_or(""), 12),
        download_count: attributes["download_count"].as_u64().unwrap_or(0),
    })
}

fn bounded_text(text: &str, limit: usize) -> String {
    text.chars()
        .filter(|character| !character.is_control())
        .take(limit)
        .collect()
}

fn safe_extension(file_name: &str) -> Option<&'static str> {
    match Path::new(file_name)
        .extension()?
        .to_str()?
        .to_ascii_lowercase()
        .as_str()
    {
        "srt" => Some("srt"),
        "ass" => Some("ass"),
        "ssa" => Some("ssa"),
        "vtt" => Some("vtt"),
        _ => None,
    }
}

fn safe_file_name(file_name: &str, extension: &str) -> Result<String, String> {
    let name = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Subtitle provider returned an invalid file name")?;
    if name.len() > 180
        || name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b' '))
        || !name
            .to_ascii_lowercase()
            .ends_with(&format!(".{extension}"))
    {
        return Err("Subtitle provider returned an unsafe file name".into());
    }
    Ok(name.into())
}

fn validate_subtitle(bytes: &[u8], extension: &str) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Subtitle file is not valid UTF-8")?;
    let text = text.trim_start_matches('\u{feff}').trim();
    let valid = match extension {
        "vtt" => text.starts_with("WEBVTT"),
        "ass" | "ssa" => text.contains("[Script Info]") && text.contains("Dialogue:"),
        "srt" => text.lines().any(|line| {
            let Some((start, end)) = line.split_once("-->") else {
                return false;
            };
            start.contains(':') && end.contains(':')
        }),
        _ => false,
    };
    if !valid || text.chars().any(|character| character == '\0') {
        return Err("Downloaded file does not look like a valid subtitle".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        SubtitleProvider, parse_result, provider_error, safe_extension, safe_file_name,
        valid_language, validate_credentials, validate_subtitle,
    };
    use crate::HttpClient;
    use std::sync::atomic::AtomicBool;
    use url::Url;

    #[derive(Clone)]
    struct FixedHttp {
        get_bytes: Vec<u8>,
        post_bytes: Vec<u8>,
    }
    impl HttpClient for FixedHttp {
        fn get(&self, _url: &Url, _cancel: &AtomicBool) -> Result<Vec<u8>, String> {
            Ok(self.get_bytes.clone())
        }
        fn get_with_headers(
            &self,
            _url: &Url,
            _headers: &[(&str, &str)],
            _cancel: &AtomicBool,
        ) -> Result<Vec<u8>, String> {
            Ok(self.get_bytes.clone())
        }
        fn post_json(
            &self,
            _url: &Url,
            _headers: &[(&str, &str)],
            _body: &[u8],
            _cancel: &AtomicBool,
        ) -> Result<Vec<u8>, String> {
            Ok(self.post_bytes.clone())
        }
    }

    fn subtitle(file_id: &str, file_name: &str) -> super::SubtitleResult {
        super::SubtitleResult {
            file_id: file_id.into(),
            file_name: file_name.into(),
            title: "Film".into(),
            release: String::new(),
            language: "en".into(),
            download_count: 0,
        }
    }

    #[test]
    fn provider_requires_encrypted_remote_origins_and_valid_api_keys() {
        assert!(SubtitleProvider::new("http://subtitles.example/api/v1/").is_err());
        assert!(SubtitleProvider::new("https://user:pass@subtitles.example/api/v1/").is_err());
        assert!(SubtitleProvider::new("https://subtitles.example/api/v1/?token=x").is_err());
        assert!(validate_credentials("").is_err());
        assert!(validate_credentials("key with spaces").is_err());
        assert!(validate_credentials(&"k".repeat(257)).is_err());
        assert!(validate_credentials("api_key-123").is_ok());
    }

    #[test]
    fn search_response_keeps_safe_supported_file_metadata_only() {
        let row = serde_json::json!({"attributes": {
            "language": "en", "release": "Film\n1080p", "download_count": 12,
            "feature_details": {"title": "Film\u{0000} Title"},
            "files": [{"file_id": 123, "file_name": "Film.en.srt"}]
        }});
        let parsed = parse_result(&row).unwrap();
        assert_eq!(parsed.file_id, "123");
        assert_eq!(parsed.file_name, "Film.en.srt");
        assert_eq!(parsed.title, "Film Title");
        assert_eq!(parsed.release, "Film1080p");
        assert_eq!(parsed.download_count, 12);
        assert!(parse_result(&serde_json::json!({"attributes":{"files":[]}})).is_none());
        assert!(parse_result(&serde_json::json!({"attributes":{"files":[{"file_id":1,"file_name":"archive.zip"}]}})).is_none());
    }

    #[test]
    fn subtitle_files_are_limited_to_valid_text_formats_and_safe_names() {
        assert_eq!(safe_extension("Film.en.SRT"), Some("srt"));
        assert_eq!(safe_extension("Film.en.vtt"), Some("vtt"));
        assert_eq!(safe_extension("film.zip"), None);
        assert_eq!(
            safe_file_name("../../Film.en.vtt", "vtt").unwrap(),
            "Film.en.vtt"
        );
        assert!(safe_file_name("Film\n.en.vtt", "vtt").is_err());
        assert!(safe_file_name(&"a".repeat(200), "vtt").is_err());
        assert!(validate_subtitle(b"<html>login</html>", "srt").is_err());
        assert!(validate_subtitle(b"\xff", "srt").is_err());
        assert!(validate_subtitle(b"WEBVTT\0invalid", "vtt").is_err());
        assert!(validate_subtitle(b"[Script Info]\nDialogue: hello", "ass").is_ok());
        assert!(validate_subtitle(b"1\n00:00:01 --> 00:00:02\nHello\n\n", "srt").is_ok());
    }

    #[test]
    fn language_validation_and_provider_failures_are_actionable() {
        assert!(valid_language("en"));
        assert!(valid_language("eng"));
        assert!(!valid_language("English"));
        assert!(!valid_language("EN"));
        assert!(provider_error("HTTP 429 Too Many Requests".into()).contains("rate limit"));
        assert!(provider_error("HTTP 403 Forbidden".into()).contains("API key"));
        assert!(provider_error("timed out".into()).contains("timed out"));
        assert!(provider_error("network unavailable".into()).contains("unavailable"));
    }

    #[test]
    fn download_rejects_invalid_ids_links_content_and_overwrites() {
        let provider = SubtitleProvider::opensubtitles();
        assert_eq!(provider.base.host_str(), Some("api.opensubtitles.com"));
        let cancel = AtomicBool::new(false);
        let directory =
            std::env::temp_dir().join(format!("foxide-subtitles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let valid = FixedHttp {
            get_bytes: b"WEBVTT\n".to_vec(),
            post_bytes: br#"{"link":"https://cdn.example/sub.vtt"}"#.to_vec(),
        };
        assert!(
            provider
                .download_to(
                    &valid,
                    "key",
                    &subtitle("abc", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .is_err()
        );
        assert!(
            provider
                .download_to(
                    &valid,
                    "key",
                    &subtitle("123", "Film.zip"),
                    &directory,
                    &cancel
                )
                .is_err()
        );

        let malformed = FixedHttp {
            post_bytes: b"not-json".to_vec(),
            ..valid.clone()
        };
        assert!(
            provider
                .download_to(
                    &malformed,
                    "key",
                    &subtitle("123", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .unwrap_err()
                .contains("invalid download response")
        );
        let no_link = FixedHttp {
            get_bytes: Vec::new(),
            post_bytes: br#"{"remaining":2}"#.to_vec(),
        };
        assert!(
            provider
                .download_to(
                    &no_link,
                    "key",
                    &subtitle("123", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .unwrap_err()
                .contains("no download link")
        );
        let insecure = FixedHttp {
            get_bytes: Vec::new(),
            post_bytes: br#"{"link":"http://cdn.example/sub.vtt"}"#.to_vec(),
        };
        assert!(
            provider
                .download_to(
                    &insecure,
                    "key",
                    &subtitle("123", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .unwrap_err()
                .contains("HTTPS")
        );
        let invalid_content = FixedHttp {
            get_bytes: b"<html>error</html>".to_vec(),
            post_bytes: br#"{"link":"https://cdn.example/sub.vtt"}"#.to_vec(),
        };
        assert!(
            provider
                .download_to(
                    &invalid_content,
                    "key",
                    &subtitle("123", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .unwrap_err()
                .contains("valid subtitle")
        );

        let saved = provider
            .download_to(
                &valid,
                "key",
                &subtitle("123", "Film.vtt"),
                &directory,
                &cancel,
            )
            .unwrap();
        assert!(
            provider
                .download_to(
                    &valid,
                    "key",
                    &subtitle("123", "Film.vtt"),
                    &directory,
                    &cancel
                )
                .unwrap_err()
                .contains("already exists")
        );
        std::fs::remove_dir_all(directory).unwrap();
        assert!(saved.ends_with("Film.vtt"));
    }

    #[test]
    fn search_rejects_large_and_malformed_provider_responses() {
        let provider = SubtitleProvider::opensubtitles();
        let cancel = AtomicBool::new(false);
        let large = FixedHttp {
            get_bytes: vec![b'x'; 2 * 1024 * 1024 + 1],
            post_bytes: Vec::new(),
        };
        assert!(
            provider
                .search(&large, "key", "Film", "en", &cancel)
                .unwrap_err()
                .contains("too large")
        );
        let malformed = FixedHttp {
            get_bytes: b"not-json".to_vec(),
            post_bytes: Vec::new(),
        };
        assert!(
            provider
                .search(&malformed, "key", "Film", "en", &cancel)
                .unwrap_err()
                .contains("invalid JSON")
        );
        let missing = FixedHttp {
            get_bytes: br#"{"status":200}"#.to_vec(),
            post_bytes: Vec::new(),
        };
        assert!(
            provider
                .search(&missing, "key", "Film", "en", &cancel)
                .unwrap_err()
                .contains("missing results")
        );
    }
}
