//! Opt-in owner-device acceptance, separate from hosted/offline test coverage.
//! Protocol assertions cannot establish displayed video. Record owner observation separately.
//! For viewing, use a sufficiently long silent clip and FOXIDE_CAST_VIEW_SECONDS.
//! Identifiers and pins must stay in ignored data.
#![cfg(feature = "device-integration")]
use player_cast::{CastOptions, CastPlayback, CastState, CertificatePin};
use std::{
    io,
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, Instant},
};

fn failure(message: &'static str) -> io::Error {
    io::Error::other(message)
}
fn advancing(playback: &mut CastPlayback, after: f64) -> io::Result<()> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let status = playback.status()?;
        if status.state == CastState::Playing && status.position > after {
            return Ok(());
        }
        if Instant::now() >= until {
            return Err(failure("Receiver playback clock did not advance"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn exercise(playback: &mut CastPlayback, viewing_seconds: u64) -> io::Result<()> {
    if viewing_seconds > 0
        && playback
            .last_status()
            .duration
            .is_some_and(|duration| duration < (viewing_seconds + 5) as f64)
    {
        return Err(failure(
            "Choose a clip longer than the viewing window plus five seconds",
        ));
    }
    advancing(playback, 0.2)?;
    // Protocol progress is not display proof. Leave time for a human to watch the TV.
    let viewing_until = Instant::now() + Duration::from_secs(viewing_seconds);
    let mut previous = playback.last_status().position;
    let mut last_advance = Instant::now();
    while Instant::now() < viewing_until {
        std::thread::sleep(Duration::from_secs(1));
        let status = playback.status()?;
        println!(
            "[device] viewing state={:?} position={}",
            status.state, status.position
        );
        if status.state != CastState::Playing {
            return Err(failure(
                "Receiver stopped during the visible-playback window",
            ));
        }
        if status.position > previous {
            previous = status.position;
            last_advance = Instant::now();
        } else if last_advance.elapsed() > Duration::from_secs(5) {
            return Err(failure("Receiver clock stalled during the viewing window"));
        }
    }
    if playback.pause()?.state != CastState::Paused {
        return Err(failure("Receiver did not confirm pause"));
    }
    let target = 1.0;
    playback.seek(target)?;
    playback.play()?;
    advancing(playback, target + 0.1)
}
#[test]
#[ignore = "requires an explicitly paired owner receiver and silent fixture"]
fn receiver_protocol_load_pause_seek_resume_and_owned_stop() {
    let viewing_seconds = std::env::var("FOXIDE_CAST_VIEW_SECONDS")
        .map(|value| {
            value
                .parse::<u64>()
                .expect("Viewing seconds must be an integer")
        })
        .unwrap_or(30);
    assert!(
        viewing_seconds <= 120,
        "Viewing window must be at most 120 seconds"
    );
    let address: SocketAddr = std::env::var("FOXIDE_CAST_TARGET")
        .expect("Set the private receiver socket address")
        .parse()
        .expect("Invalid receiver address");
    let pin: CertificatePin = std::env::var("FOXIDE_CAST_PIN")
        .expect("Set the privately trusted certificate fingerprint")
        .parse()
        .expect("Invalid certificate fingerprint");
    let media = PathBuf::from(
        std::env::var("FOXIDE_CAST_MEDIA").expect("Set the absolute silent fixture path"),
    );
    let mut options = CastOptions::default();
    // Explicit opt-in for repeating this owner test after the default receiver stays open.
    // Never replace another application, even when the repeat flag is set.
    if std::env::var("FOXIDE_CAST_REPEAT_DEFAULT_APP").as_deref() == Ok("1") {
        let current = player_cast::receiver_status(address, pin, Duration::from_secs(5))
            .expect("Could not inspect the current receiver application");
        options.allow_replacing_application =
            current["applications"].as_array().is_some_and(|apps| {
                !apps.is_empty() && apps.iter().all(|app| app["appId"] == "CC1AD845")
            });
    }
    let mut playback = CastPlayback::start_file(&media, address, pin, options)
        .expect("Owner receiver rejected safe-start playback");
    println!(
        "[device] initial state={:?} position={} duration={:?}",
        playback.last_status().state,
        playback.last_status().position,
        playback.last_status().duration
    );
    let exercised = exercise(&mut playback, viewing_seconds);
    // Always attempt owned stop before reporting a failure; never force takeover.
    println!("[device] transfer={:?}", playback.transfer_stats());
    let stopped = playback.stop();
    exercised.expect("Physical receiver playback controls failed");
    stopped.expect("Physical receiver did not acknowledge owned stop");
    println!(
        "[device] Protocol assertions only: displayed video requires a separate owner observation."
    );
    println!(
        "{{\"protocol_checks\":\"passed\",\"physical_playback\":\"unverified\",\"load\":true,\"clock_advanced\":true,\"pause\":true,\"seek\":true,\"resume\":true,\"owned_stop\":true,\"display_confirmation\":\"pending\"}}"
    );
}
