use player_services::{HttpClient, SubtitleProvider};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use url::Url;

struct FakeHttp {
    requests: Mutex<Vec<(String, String)>>,
}

impl HttpClient for FakeHttp {
    fn get(&self, url: &Url, _cancel: &AtomicBool) -> Result<Vec<u8>, String> {
        let mut requests = self.requests.lock().unwrap();
        requests.push(("GET".into(), url.to_string()));
        if url.host_str() == Some("cdn.example") {
            return Ok(b"WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello\n".to_vec());
        }
        Ok(br#"{"data":[{"attributes":{"language":"en","release":"Film.2025.1080p","download_count":42,"feature_details":{"title":"Film"},"files":[{"file_id":123,"file_name":"Film.en.vtt"}]}}]}"#.to_vec())
    }

    fn get_with_headers(
        &self,
        url: &Url,
        headers: &[(&str, &str)],
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        assert!(
            headers
                .iter()
                .any(|(name, value)| *name == "Api-Key" && *value == "user-key")
        );
        self.get(url, cancel)
    }

    fn post_json(
        &self,
        url: &Url,
        headers: &[(&str, &str)],
        body: &[u8],
        _cancel: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        assert!(
            headers
                .iter()
                .any(|(name, value)| *name == "Api-Key" && *value == "user-key")
        );
        assert_eq!(body, br#"{"file_id":123}"#);
        self.requests
            .lock()
            .unwrap()
            .push(("POST".into(), url.to_string()));
        Ok(br#"{"link":"https://cdn.example/subtitle.vtt","file_name":"Film.en.vtt","remaining":9}"#.to_vec())
    }
}

#[test]
fn user_can_search_choose_and_download_a_subtitle() {
    let http = FakeHttp {
        requests: Mutex::new(Vec::new()),
    };
    let cancel = AtomicBool::new(false);
    let provider = SubtitleProvider::new("https://api.example/api/v1/").unwrap();
    let results = provider
        .search(&http, "user-key", "Film", "en", &cancel)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Film");
    assert_eq!(results[0].release, "Film.2025.1080p");
    assert_eq!(results[0].language, "en");
    let directory = std::env::temp_dir().join(format!("foxide-subtitles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = provider
        .download_to(&http, "user-key", &results[0], &directory, &cancel)
        .unwrap();
    assert_eq!(path.extension().unwrap(), "vtt");
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.starts_with("WEBVTT"));
    assert!(
        http.requests
            .lock()
            .unwrap()
            .iter()
            .any(|(method, _)| method == "POST")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn invalid_inputs_and_cancelled_requests_never_reach_provider() {
    let http = FakeHttp {
        requests: Mutex::new(Vec::new()),
    };
    let cancel = AtomicBool::new(false);
    let provider = SubtitleProvider::new("https://api.example/api/v1/").unwrap();
    assert!(provider.search(&http, "", "Film", "en", &cancel).is_err());
    assert!(
        provider
            .search(&http, "user-key", " ", "en", &cancel)
            .is_err()
    );
    assert!(
        provider
            .search(&http, "user-key", "Film", "English", &cancel)
            .is_err()
    );
    assert!(http.requests.lock().unwrap().is_empty());

    cancel.store(true, Ordering::Release);
    assert!(
        provider
            .search(&http, "user-key", "Film", "en", &cancel)
            .is_err()
    );
    assert!(http.requests.lock().unwrap().is_empty());
}

