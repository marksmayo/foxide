//! Opt-in owner-device acceptance, separate from hosted/offline test coverage.
//! Use a short silent local video. Identifiers and pins must stay in ignored data.
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
fn exercise(playback: &mut CastPlayback) -> io::Result<()> {
    advancing(playback, 0.2)?;
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
fn owner_receiver_load_pause_seek_resume_and_owned_stop() {
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
    let mut playback = CastPlayback::start_file(&media, address, pin, CastOptions::default())
        .expect("Owner receiver rejected safe-start playback");
    println!(
        "[device] initial state={:?} position={} duration={:?}",
        playback.last_status().state,
        playback.last_status().position,
        playback.last_status().duration
    );
    let exercised = exercise(&mut playback);
    // Always attempt owned stop before reporting a failure; never force takeover.
    println!("[device] transfer={:?}", playback.transfer_stats());
    let stopped = playback.stop();
    exercised.expect("Physical receiver playback controls failed");
    stopped.expect("Physical receiver did not acknowledge owned stop");
    println!(
        "{{\"load\":true,\"clock_advanced\":true,\"pause\":true,\"seek\":true,\"resume\":true,\"owned_stop\":true,\"display_confirmation\":\"pending\"}}"
    );
}
