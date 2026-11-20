use player_services::{HttpClient, RadioDirectory};

struct DirectoryResponse;
impl HttpClient for DirectoryResponse {
    fn get(
        &self,
        url: &url::Url,
        _cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<u8>, String> {
        assert_eq!(url.host_str(), Some("radio.example.org"));
        let query: std::collections::BTreeMap<_, _> = url.query_pairs().collect();
        assert_eq!(query.get("name").unwrap(), "jazz & soul");
        assert_eq!(query.get("limit").unwrap(), "50");
        assert_eq!(query.get("hidebroken").unwrap(), "true");
        Ok(br#"[{"stationuuid":"abc","name":"Example Jazz","url_resolved":"https://stream.example.org/live","country":"NZ","codec":"AAC","bitrate":128,"lastcheckok":1},{"stationuuid":"bad","name":"Unsafe","url_resolved":"file:///private/media","lastcheckok":1}]"#.to_vec())
    }
}

#[test]
fn radio_search_encodes_queries_and_returns_only_playable_station_urls() {
    let directory = RadioDirectory::new("https://radio.example.org").unwrap();
    let stations = directory
        .search(
            &DirectoryResponse,
            "jazz & soul",
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(stations.len(), 1);
    assert_eq!(stations[0].name, "Example Jazz");
    assert_eq!(stations[0].url.as_str(), "https://stream.example.org/live");
}

struct DiscoverDirectory;
impl HttpClient for DiscoverDirectory {
    fn get(&self, url: &url::Url, _: &std::sync::atomic::AtomicBool) -> Result<Vec<u8>, String> {
        if url.path() == "/json/servers" {
            return Ok(br#"[{"name":"nl1.api.radio-browser.info"},{"name":"nl1.api.radio-browser.info"},{"name":"evil.example.org"},{"name":"x.api.radio-browser.info@evil.example.org"}]"#.to_vec());
        }
        assert_eq!(url.host_str(), Some("nl1.api.radio-browser.info"));
        Ok(br#"[{"stationuuid":"abc","name":"Discovered","url_resolved":"https://stream.example.org/live","lastcheckok":1}]"#.to_vec())
    }
}

#[test]
fn public_radio_directory_discovers_current_mirrors_and_rejects_outside_hosts() {
    let stations = RadioDirectory::public()
        .search(
            &DiscoverDirectory,
            "jazz",
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(stations[0].name, "Discovered");
}

#[derive(Default)]
struct CountDiscovery {
    lookups: std::sync::atomic::AtomicUsize,
    searches: std::sync::atomic::AtomicUsize,
}
impl HttpClient for CountDiscovery {
    fn get(
        &self,
        url: &url::Url,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<u8>, String> {
        if url.path() == "/json/servers" {
            self.lookups
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        } else {
            self.searches
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        DiscoverDirectory.get(url, cancel)
    }
}

#[test]
fn repeated_searches_share_mirror_discovery_but_fetch_fresh_station_results() {
    let directory = RadioDirectory::public();
    let client = CountDiscovery::default();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    directory.search(&client, "first", &cancel).unwrap();
    directory
        .clone()
        .search(&client, "second", &cancel)
        .unwrap();
    assert_eq!(client.lookups.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(client.searches.load(std::sync::atomic::Ordering::SeqCst), 2);
}

struct Response(Vec<u8>);
impl HttpClient for Response {
    fn get(&self, _: &url::Url, _: &std::sync::atomic::AtomicBool) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}
struct NoRequests;
impl HttpClient for NoRequests {
    fn get(&self, _: &url::Url, _: &std::sync::atomic::AtomicBool) -> Result<Vec<u8>, String> {
        panic!("Invalid/cancelled input must not issue a request")
    }
}

#[test]
fn radio_rejects_bad_origins_queries_and_responses_without_network_side_effects() {
    for origin in [
        "file:///private",
        "http://example.org",
        "https://user:pass@example.org",
        "https://example.org/path",
        "https://example.org/?q=x",
        "https://example.org/#x",
        "not a url",
    ] {
        assert!(RadioDirectory::new(origin).is_err(), "accepted {origin}");
    }
    assert!(RadioDirectory::new("http://localhost:1234").is_ok());
    assert!(RadioDirectory::new("http://[::1]:1234").is_ok());
    let directory = RadioDirectory::new("https://radio.example.org").unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for query in ["".to_owned(), " ".to_owned(), "x".repeat(129)] {
        assert!(directory.search(&NoRequests, &query, &cancel).is_err());
    }
    assert!(
        directory
            .search(
                &NoRequests,
                "jazz",
                &std::sync::atomic::AtomicBool::new(true)
            )
            .is_err()
    );
    assert!(
        RadioDirectory::public()
            .search(
                &NoRequests,
                "jazz",
                &std::sync::atomic::AtomicBool::new(true)
            )
            .is_err()
    );
    for body in [
        b"not-json".to_vec(),
        b"{}".to_vec(),
        vec![b' '; 2 * 1024 * 1024 + 1],
        serde_json::to_vec(&vec![serde_json::Value::Null; 1001]).unwrap(),
    ] {
        assert!(directory.search(&Response(body), "jazz", &cancel).is_err());
    }
    assert!(directory.click(&NoRequests, "../bad", &cancel).is_err());
    directory
        .click(&Response(Vec::new()), "valid-id", &cancel)
        .unwrap();
}

#[test]
fn directory_filters_unusable_rows_and_caps_the_visible_station_count() {
    let good = serde_json::json!({"stationuuid":"id-1","name":"Clean\nName","url_resolved":"https://stream.example.org/live","lastcheckok":1});
    let mut values = vec![
        serde_json::json!({"stationuuid":"bad","name":"broken","url_resolved":"https://x.example.org","lastcheckok":0}),
        serde_json::json!({"stationuuid":"bad/id","name":"bad","url_resolved":"https://x.example.org","lastcheckok":1}),
        serde_json::json!({"stationuuid":"id","name":"bad","url_resolved":"https://user:secret@x.example.org","lastcheckok":1}),
        serde_json::json!({"stationuuid":"id","name":"x".repeat(513),"url_resolved":"https://x.example.org","lastcheckok":1}),
        serde_json::json!({"name":"missing fields"}),
    ];
    values.extend(vec![good; 60]);
    let rows = RadioDirectory::new("https://radio.example.org")
        .unwrap()
        .search(
            &Response(serde_json::to_vec(&values).unwrap()),
            "jazz",
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0].name, "Clean Name");
}

#[test]
fn public_discovery_has_bounded_retries_and_handles_malformed_provider_metadata() {
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for body in [
        b"invalid".to_vec(),
        b"{}".to_vec(),
        b"[]".to_vec(),
        vec![b' '; 2 * 1024 * 1024 + 1],
        serde_json::to_vec(&vec![serde_json::json!({"name":"bad"}); 65]).unwrap(),
    ] {
        assert!(
            RadioDirectory::public()
                .search(&Response(body), "jazz", &cancel)
                .is_err()
        );
    }
    struct Down(std::sync::atomic::AtomicUsize);
    impl HttpClient for Down {
        fn get(
            &self,
            url: &url::Url,
            _: &std::sync::atomic::AtomicBool,
        ) -> Result<Vec<u8>, String> {
            if url.path() == "/json/servers" {
                return Ok(br#"[{"name":"one.api.radio-browser.info"},{"name":"two.api.radio-browser.info"},{"name":"three.api.radio-browser.info"},{"name":"four.api.radio-browser.info"}]"#.to_vec());
            }
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err("mirror unavailable".into())
        }
    }
    let down = Down(Default::default());
    assert!(
        RadioDirectory::public()
            .search(&down, "jazz", &cancel)
            .is_err()
    );
    assert_eq!(down.0.load(std::sync::atomic::Ordering::SeqCst), 3);
}
