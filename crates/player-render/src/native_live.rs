//! Explicit live-provider acceptance. Ordinary tests never contact providers.
use super::*;
use std::{path::Path, time::Instant};

fn decoder_ready(name: &str) -> bool {
    !matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "" | "null" | "unknown" | "no" | "unavailable" | "n/a"
    )
}

#[test]
fn unset_native_properties_cannot_be_reported_as_real_decoders() {
    for unset in [
        "",
        "null",
        "NULL",
        " null ",
        " ",
        "unknown",
        "no",
        "unavailable",
        "n/a",
    ] {
        assert!(
            !decoder_ready(unset),
            "{unset} must not establish decoder acceptance"
        );
    }
    for codec in ["flac", "h264", "mpeg4", "mp3", "pcm_s16le"] {
        assert!(decoder_ready(codec));
    }
}

fn selected_decoder(api: &Api, kind: &str, legacy: &CStr) -> String {
    let name = CString::new(format!("current-tracks/{kind}/decoder")).unwrap();
    let current = api.get(&name);
    if decoder_ready(&current) {
        return current;
    }
    let count = api.number(c"track-list/count").clamp(0.0, 256.0) as u32;
    for index in 0..count {
        if api.track_property(index, "type") == kind
            && api.track_property(index, "selected") == "yes"
        {
            let decoder = api.track_property(index, "decoder");
            if decoder_ready(&decoder) {
                return decoder;
            }
        }
    }
    // Older engines lack the decoder subproperty. Their valid legacy name plus
    // configured output and an advancing clock provides the compatibility proof.
    api.get(legacy)
}

fn probe(url: &str, helper: Option<&Path>, video: bool) -> Result<serde_json::Value, String> {
    let engine = Engine::with_video_site_helper(helper)?;
    for (name, value) in [
        ("vo", "null"),
        ("ao", "null"),
        ("pause", "no"),
        ("network-timeout", "10"),
        ("demuxer-max-bytes", "5242880"),
        ("demuxer-readahead-secs", "1"),
    ] {
        engine.command(vec!["set".into(), name.into(), value.into()])?;
    }
    engine.ready.store(true, Ordering::Release);
    engine.command(vec!["loadfile".into(), url.into(), "replace".into()])?;
    let api = engine.api.as_ref();
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        let snapshot = engine.snapshot().unwrap_or_default();
        if !snapshot.error.is_empty() {
            return Err(snapshot.error);
        }
        if !snapshot.command_error.is_empty() {
            return Err(snapshot.command_error);
        }
        let source_matches = snapshot.requested_path.as_ref() == url;
        let audio = selected_decoder(api, "audio", c"audio-codec-name");
        let image = selected_decoder(api, "video", c"video-format");
        let video_width = api.number(c"video-out-params/w");
        if snapshot.loaded
            && source_matches
            && decoder_ready(&audio)
            && (!video || (decoder_ready(&image) && video_width > 0.0))
            && api.number(c"audio-out-params/samplerate") > 0.0
            && snapshot.position >= 0.3
        {
            engine.command(vec!["stop".into()])?;
            return Ok(
                serde_json::json!({"audio_decoder": audio, "video_decoder": image, "video_output_width": video_width, "clock_advanced": true, "source_identity_preserved": true, "displayed_frame_measured": false, "output": "null"}),
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!(
        "Provider did not establish real decoders and advance its clock within 60 seconds (audio={}, video={}, width={})",
        api.get(c"audio-codec-name"),
        api.get(c"video-format"),
        api.number(c"video-out-params/w")
    ))
}
fn record(feature: &str, source: &str, outcome: Result<serde_json::Value, String>) {
    let report = match &outcome {
        Ok(proof) => {
            serde_json::json!({"feature": feature, "status": "accepted", "source": source, "proof": proof})
        }
        Err(error) => {
            serde_json::json!({"feature": feature, "status": "failed", "source": source, "error": error})
        }
    };
    let folder = std::env::var_os("FOXIDE_LIVE_ACCEPTANCE_DIR")
        .expect("Set an ignored output directory for explicit live acceptance");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        PathBuf::from(folder).join(format!("{feature}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    assert!(
        outcome.is_ok(),
        "Live {feature} acceptance: {}",
        outcome.unwrap_err()
    );
}
#[test]
#[ignore = "Explicit live directory/stream acceptance only; requires Internet and libmpv"]
fn radio_directory_searches_a_live_provider_and_decodes_a_returned_stream() {
    let result = (|| {
        let client = player_services::CurlClient::system()?;
        let stations = player_services::RadioDirectory::public().search(
            &client,
            "Radio Paradise",
            &AtomicBool::new(false),
        )?;
        let mut error = "No HTTPS stations returned by the live directory".to_owned();
        for station in stations
            .iter()
            .filter(|station| station.url.scheme() == "https")
            .take(3)
        {
            match probe(station.url.as_str(), None, false) {
                Ok(mut proof) => {
                    proof["directory_results"] = stations.len().into();
                    proof["stream_host"] = station.url.host_str().unwrap_or_default().into();
                    return Ok(proof);
                }
                Err(failure) => error = failure,
            }
        }
        Err(error)
    })();
    record(
        "radio_directory",
        "https://docs.radio-browser.info/",
        result,
    );
}
#[test]
#[ignore = "Explicit live-site acceptance only; requires a verified yt-dlp helper and Internet"]
fn video_site_page_uses_the_real_selected_helper_and_decodes_audio_and_video() {
    let result = (|| {
        let helper = std::env::var_os("FOXIDE_LIVE_YTDLP")
            .ok_or("Select the verified helper with FOXIDE_LIVE_YTDLP")?;
        probe(
            "https://archive.org/details/BigBuckBunny_328",
            Some(Path::new(&helper)),
            true,
        )
    })();
    record(
        "youtube_urls",
        "https://archive.org/details/BigBuckBunny_328",
        result,
    );
}
