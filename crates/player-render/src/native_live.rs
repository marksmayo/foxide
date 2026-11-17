//! Explicit live-provider acceptance. Ordinary tests never contact providers.
use super::*;
use std::{path::Path, time::Instant};

fn probe(url: &str, helper: Option<&Path>, video: bool) -> Result<serde_json::Value, String> {
    let selected = helper
        .map(player_services::VideoSiteTool::new)
        .transpose()?;
    let api = Api::load(selected.as_ref())?;
    for (name, value) in [
        ("vo", "null"),
        ("ao", "null"),
        ("pause", "no"),
        ("network-timeout", "10"),
        ("demuxer-max-bytes", "5242880"),
        ("demuxer-readahead-secs", "1"),
    ] {
        api.send(&["set".into(), name.into(), value.into()])?;
    }
    api.send(&["loadfile".into(), url.into(), "replace".into()])?;
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut restarted = false;
    while Instant::now() < deadline {
        // SAFETY: this test owns the client; event contents are read before the next wait.
        unsafe {
            let event = (api.wait_event)(api.handle, 0.1);
            if !event.is_null() {
                if (*event).kind == 21 {
                    restarted = true;
                }
                if (*event).kind == 7 && !(*event).data.is_null() {
                    let end = &*((*event).data.cast::<End>());
                    if end.reason == 4 {
                        return Err(format!(
                            "Provider media failed to load (engine error {})",
                            end.error
                        ));
                    }
                }
            }
        }
        let audio = api.get(c"audio-codec-name");
        let image = api.get(c"video-format");
        let video_width = api.number(c"video-out-params/w");
        if restarted
            && !audio.is_empty()
            && (!video || (!image.is_empty() && video_width > 0.0))
            && api.number(c"time-pos") >= 0.3
        {
            api.send(&["stop".into()])?;
            return Ok(
                serde_json::json!({"audio_decoder": audio, "video_decoder": image, "video_output_width": video_width, "clock_advanced": true, "displayed_frame_measured": false, "output": "null"}),
            );
        }
    }
    Err("Provider did not decode and advance its playback clock within 60 seconds".into())
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
            "ytdl://https://archive.org/details/BigBuckBunny_328",
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
