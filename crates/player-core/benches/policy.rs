//! CPU-only playback policy; no decoder, process launch, GPU or media I/O.
use divan::{Bencher, black_box};
use player_core::{PlaybackOptions, classify_decode_path, external_path};
use std::path::PathBuf;

fn main() {
    divan::main();
}

#[divan::bench(args = [0.25, 0.5, 1.0, 1.5, 2.0, 4.0])]
fn speed_arguments(bencher: Bencher, speed: f64) {
    let options = PlaybackOptions {
        speed,
        ..Default::default()
    };
    assert!(options.mpv_args().is_ok());
    bencher.bench_local(|| black_box(&options).mpv_args().unwrap());
}

#[divan::bench(args = [0, 1, 8, 32, 128])]
fn subtitle_arguments(bencher: Bencher, count: usize) {
    let options = PlaybackOptions {
        subtitle_files: (0..count)
            .map(|i| PathBuf::from(format!("/media/Série/字幕 {i:03}.ass")))
            .collect(),
        ..Default::default()
    };
    let args = options.mpv_args().unwrap();
    assert_eq!(
        args.iter()
            .filter(|a| a.to_string_lossy().starts_with("--sub-file="))
            .count(),
        count
    );
    bencher.bench_local(|| black_box(&options).mpv_args().unwrap());
}

#[divan::bench(args = ["audio", "subtitle", "sync", "picture", "deinterlace", "tone_map", "start", "chapter", "resume", "minimal", "loop", "ab_loop", "paused"])]
fn feature_arguments(bencher: Bencher, feature: &str) {
    let mut options = PlaybackOptions::default();
    match feature {
        "audio" => options.audio_track = Some(2),
        "subtitle" => options.subtitle_track = Some(3),
        "sync" => {
            options.audio_delay = -0.25;
            options.subtitle_delay = 0.5;
        }
        "picture" => {
            options.brightness = 10;
            options.contrast = -5;
            options.gamma = 3;
        }
        "deinterlace" => options.deinterlace = true,
        "tone_map" => options.tone_map = true,
        "start" => options.start = Some(3600.25),
        "chapter" => options.chapter = Some(12),
        "resume" => options.resume_dir = Some("/state/Foxide/watch later".into()),
        "minimal" => {
            options.minimal = true;
            options.on_top = true;
        }
        "loop" => options.loop_playlist = true,
        "ab_loop" => options.ab_loop = Some((10.0, 20.0)),
        "paused" => options.paused = true,
        _ => unreachable!(),
    }
    assert!(options.mpv_args().is_ok());
    bencher.bench_local(|| black_box(&options).mpv_args().unwrap());
}

#[divan::bench(args = ["speed_nan", "speed_low", "speed_high", "audio_inf", "subtitle_nan", "audio_zero", "subtitle_zero", "brightness_high", "contrast_low", "gamma_high", "start_negative", "start_nan", "start_chapter", "ab_reversed", "ab_negative", "ab_nan"])]
fn rejected_arguments(bencher: Bencher, invalid: &str) {
    let mut options = PlaybackOptions::default();
    match invalid {
        "speed_nan" => options.speed = f64::NAN,
        "speed_low" => options.speed = 0.1,
        "speed_high" => options.speed = 4.1,
        "audio_inf" => options.audio_delay = f64::INFINITY,
        "subtitle_nan" => options.subtitle_delay = f64::NAN,
        "audio_zero" => options.audio_track = Some(0),
        "subtitle_zero" => options.subtitle_track = Some(0),
        "brightness_high" => options.brightness = 101,
        "contrast_low" => options.contrast = -101,
        "gamma_high" => options.gamma = 101,
        "start_negative" => options.start = Some(-1.0),
        "start_nan" => options.start = Some(f64::NAN),
        "start_chapter" => {
            options.start = Some(1.0);
            options.chapter = Some(2);
        }
        "ab_reversed" => options.ab_loop = Some((20.0, 10.0)),
        "ab_negative" => options.ab_loop = Some((-1.0, 20.0)),
        "ab_nan" => options.ab_loop = Some((0.0, f64::NAN)),
        _ => unreachable!(),
    }
    assert!(options.mpv_args().is_err());
    bencher.bench_local(|| black_box(&options).mpv_args().unwrap_err());
}

#[divan::bench(args = ["", "no", "d3d11va", "d3d11va-copy", "vaapi", "vaapi-copy", "videotoolbox", "videotoolbox-copy", "nvdec", "nvdec-copy", "vulkan", "auto-safe", "auto-copy", "unknown"])]
fn decoder_classification(bencher: Bencher, name: &str) {
    bencher.bench_local(|| classify_decode_path(black_box(Some(name))));
}

#[divan::bench]
fn missing_decoder(bencher: Bencher) {
    bencher.bench_local(|| classify_decode_path(black_box(None)));
}

#[divan::bench(args = [1, 32, 1024])]
fn decoder_observation_batch(bencher: Bencher, count: usize) {
    let inputs = [
        None,
        Some("no"),
        Some("d3d11va"),
        Some("d3d11va-copy"),
        Some("nvdec-copy"),
    ];
    let names: Vec<_> = inputs.into_iter().cycle().take(count).collect();
    bencher.bench_local(|| {
        for name in black_box(&names) {
            black_box(classify_decode_path(black_box(*name)));
        }
    });
}

#[divan::bench(args = ["/media/clip.mp4", "/media/Série/字幕 with spaces.ass", r"C:\media\clip.mp4", r"\\?\C:\media\clip.mp4", r"\\?\UNC\server\share\clip.mp4"])]
fn external_player_path(bencher: Bencher, name: &str) {
    bencher
        .with_inputs(|| PathBuf::from(name))
        .bench_local_values(|path| external_path(black_box(path)));
}
