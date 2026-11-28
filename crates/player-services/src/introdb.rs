//! Read-only, user-triggered episode marker lookups through IntroDB.
use crate::{HttpClient, public_http_url};
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

const INTRO_RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeIdentity {
    imdb_id: String,
    season: u32,
    episode: u32,
}

impl EpisodeIdentity {
    pub fn new(imdb_id: &str, season: u32, episode: u32) -> Result<Self, String> {
        let valid_id = matches!(imdb_id.len(), 9 | 10)
            && imdb_id.starts_with("tt")
            && imdb_id[2..].bytes().all(|byte| byte.is_ascii_digit());
        if !valid_id {
            return Err("IMDb episode ID must have the form tt1234567".into());
        }
        if season == 0 || episode == 0 || season > 10_000 || episode > 100_000 {
            return Err("Season and episode numbers are outside the supported range".into());
        }
        Ok(Self {
            imdb_id: imdb_id.to_owned(),
            season,
            episode,
        })
    }

    pub fn imdb_id(&self) -> &str {
        &self.imdb_id
    }

    pub fn season(&self) -> u32 {
        self.season
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntroSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub confidence: Option<f64>,
    pub submission_count: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct IntroDb {
    base: Url,
}

impl IntroDb {
    pub fn public() -> Self {
        Self {
            base: Url::parse("https://api.introdb.app/").expect("Static IntroDB API origin"),
        }
    }

    pub fn lookup_intro(
        &self,
        client: &impl HttpClient,
        identity: &EpisodeIdentity,
        cancel: &AtomicBool,
    ) -> Result<Option<IntroSegment>, String> {
        if cancel.load(Ordering::Acquire) {
            return Err("Intro lookup cancelled".into());
        }
        let mut url = self
            .base
            .join("segments")
            .map_err(|_| "Invalid IntroDB API URL")?;
        url.query_pairs_mut().extend_pairs([
            ("imdb_id", identity.imdb_id.as_str()),
            ("season", &identity.season.to_string()),
            ("episode", &identity.episode.to_string()),
        ]);
        public_http_url(url.as_str())?;
        let body = client.get(&url, cancel)?;
        if cancel.load(Ordering::Acquire) {
            return Err("Intro lookup cancelled".into());
        }
        if body.len() > INTRO_RESPONSE_LIMIT {
            return Err("IntroDB response is too large".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| "IntroDB returned invalid JSON")?;
        if value["imdb_id"].as_str() != Some(identity.imdb_id.as_str())
            || value["season"].as_u64() != Some(identity.season as u64)
            || value["episode"].as_u64() != Some(identity.episode as u64)
        {
            return Err("IntroDB response did not match the requested episode".into());
        }
        let Some(intro) = value.get("intro").filter(|intro| !intro.is_null()) else {
            return Ok(None);
        };
        let start_seconds = intro["start_sec"]
            .as_f64()
            .ok_or("IntroDB intro has no valid start time")?;
        let end_seconds = intro["end_sec"]
            .as_f64()
            .ok_or("IntroDB intro has no valid end time")?;
        if !start_seconds.is_finite()
            || !end_seconds.is_finite()
            || start_seconds < 0.0
            || end_seconds <= start_seconds
        {
            return Err("IntroDB returned invalid intro boundaries".into());
        }
        let confidence = match intro.get("confidence") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => Some(
                value
                    .as_f64()
                    .ok_or("IntroDB returned an invalid confidence value")?,
            ),
        };
        if confidence.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
            return Err("IntroDB returned an invalid confidence value".into());
        }
        Ok(Some(IntroSegment {
            start_seconds,
            end_seconds,
            confidence,
            submission_count: intro
                .get("submission_count")
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpClient;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use url::Url;

    struct Fixture {
        body: Vec<u8>,
        requested: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl HttpClient for Fixture {
        fn get(&self, url: &Url, _: &AtomicBool) -> Result<Vec<u8>, String> {
            self.requested.lock().unwrap().push(url.to_string());
            Ok(self.body.clone())
        }
    }

    struct CancelsDuringRead {
        body: Vec<u8>,
    }

    impl HttpClient for CancelsDuringRead {
        fn get(&self, _: &Url, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
            cancel.store(true, Ordering::Release);
            Ok(self.body.clone())
        }
    }

    #[test]
    fn lookup_uses_explicit_episode_identity_and_returns_intro_segment() {
        let requested = Arc::new(std::sync::Mutex::new(Vec::new()));
        let client = Fixture {
            body: br#"{"imdb_id":"tt0903747","media_type":"tv","is_movie":false,"season":1,"episode":2,"intro":{"start_sec":2.5,"end_sec":58,"confidence":0.92,"submission_count":14},"recap":null,"outro":null,"post_credits":null,"no_intro":false,"no_recap":true,"no_outro":false}"#.to_vec(),
            requested: requested.clone(),
        };
        let identity = EpisodeIdentity::new("tt0903747", 1, 2).unwrap();

        let segment = IntroDb::public()
            .lookup_intro(&client, &identity, &AtomicBool::new(false))
            .unwrap()
            .unwrap();

        assert_eq!(segment.start_seconds, 2.5);
        assert_eq!(segment.end_seconds, 58.0);
        assert_eq!(segment.confidence, Some(0.92));
        let urls = requested.lock().unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].starts_with("https://api.introdb.app/segments?"));
        assert!(urls[0].contains("imdb_id=tt0903747"));
        assert!(urls[0].contains("season=1"));
        assert!(urls[0].contains("episode=2"));
    }

    #[test]
    fn lookup_rejects_untrusted_or_malformed_segment_data() {
        for body in [
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":null}"#.as_slice(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":58,"end_sec":2.5}}"#.as_slice(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":-1,"end_sec":2.5}}"#.as_slice(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":1,"end_sec":1}}"#.as_slice(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":1e999,"end_sec":2}}"#.as_slice(),
        ] {
            let client = Fixture {
                body: body.to_vec(),
                requested: Arc::default(),
            };
            let identity = EpisodeIdentity::new("tt0903747", 1, 2).unwrap();
            let result = IntroDb::public().lookup_intro(
                &client,
                &identity,
                &AtomicBool::new(false),
            );
            if body == br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":null}"# {
                assert_eq!(result.unwrap(), None);
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn identity_validation_and_cancellation_happen_before_network_io() {
        assert!(EpisodeIdentity::new("/media/private.mkv", 1, 2).is_err());
        assert!(EpisodeIdentity::new("tt123", 1, 2).is_err());
        assert!(EpisodeIdentity::new("tt0903747", 0, 2).is_err());
        assert!(EpisodeIdentity::new("tt0903747", 1, 0).is_err());

        let requested = Arc::new(std::sync::Mutex::new(Vec::new()));
        let client = Fixture {
            body: Vec::new(),
            requested: requested.clone(),
        };
        let identity = EpisodeIdentity::new("tt0903747", 1, 2).unwrap();
        let cancel = AtomicBool::new(true);
        assert!(
            IntroDb::public()
                .lookup_intro(&client, &identity, &cancel)
                .is_err()
        );
        assert!(cancel.load(Ordering::Acquire));
        assert!(requested.lock().unwrap().is_empty());
    }

    #[test]
    fn cancellation_during_http_read_discards_a_valid_response() {
        let client = CancelsDuringRead {
            body: br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":2.5,"end_sec":58}}"#.to_vec(),
        };
        let identity = EpisodeIdentity::new("tt0903747", 1, 2).unwrap();
        let cancel = AtomicBool::new(false);

        let error = IntroDb::public()
            .lookup_intro(&client, &identity, &cancel)
            .unwrap_err();

        assert_eq!(error, "Intro lookup cancelled");
        assert!(cancel.load(Ordering::Acquire));
    }

    #[test]
    fn lookup_rejects_mismatched_episode_low_quality_and_oversized_responses() {
        let identity = EpisodeIdentity::new("tt0903747", 1, 2).unwrap();
        for body in [
            br#"{"imdb_id":"tt0903748","season":1,"episode":2,"intro":{"start_sec":2,"end_sec":20}}"#.to_vec(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":2,"end_sec":20,"confidence":1.1}}"#.to_vec(),
            br#"{"imdb_id":"tt0903747","season":1,"episode":2,"intro":{"start_sec":2,"end_sec":20,"confidence":"certain"}}"#.to_vec(),
            vec![b' '; INTRO_RESPONSE_LIMIT + 1],
        ] {
            let client = Fixture {
                body,
                requested: Arc::default(),
            };
            assert!(IntroDb::public()
                .lookup_intro(&client, &identity, &AtomicBool::new(false))
                .is_err());
        }
    }

    #[test]
    #[ignore = "requires one read from the public IntroDB API"]
    fn live_provider_response_matches_the_requested_episode() {
        let client = crate::CurlClient::system().expect("System curl is installed in CI");
        let identity = EpisodeIdentity::new("tt0903747", 1, 1).unwrap();
        let segment = IntroDb::public()
            .lookup_intro(&client, &identity, &AtomicBool::new(false))
            .expect("IntroDB returns a valid response for the requested episode");

        if let Some(segment) = segment {
            assert!(segment.start_seconds >= 0.0);
            assert!(segment.end_seconds > segment.start_seconds);
            assert!(
                segment
                    .confidence
                    .is_none_or(|value| (0.0..=1.0).contains(&value))
            );
            assert!(segment.submission_count.is_none_or(|value| value > 0));
        }
    }
}

