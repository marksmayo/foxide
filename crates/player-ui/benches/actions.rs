//! UI policy CPU benchmarks. No Slint window, compositor or presentation timing.
use divan::{Bencher, black_box};
use player_ui::{Action, Effect, PlaybackControl, UiState, shortcut};
use std::path::PathBuf;

fn main() {
    divan::main();
}

fn playlist(count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|i| PathBuf::from(format!("/media/Série/Episode {i:05}.mkv")))
        .collect()
}

fn state(count: usize, loaded: bool) -> UiState {
    let mut state = UiState {
        playlist: std::sync::Arc::new(playlist(count)),
        selected: count / 2,
        ..Default::default()
    };
    if loaded {
        state.confirm_loaded(7200.0);
    }
    state
}

fn action(name: &str) -> Action {
    match name {
        "pause" => Action::TogglePause,
        "mute" => Action::ToggleMute,
        "seek_backward" => Action::Seek(-5.0),
        "seek_forward" => Action::Seek(30.0),
        "seek_nan" => Action::Seek(f64::NAN),
        "scrub" => Action::SeekAbsolute(1234.0),
        "scrub_before_start" => Action::SeekAbsolute(-10.0),
        "scrub_past_end" => Action::SeekAbsolute(10000.0),
        "scrub_infinite" => Action::SeekAbsolute(f64::INFINITY),
        "speed_quarter" => Action::SetSpeed(0.25),
        "speed_normal" => Action::SetSpeed(1.0),
        "speed_double" => Action::SetSpeed(2.0),
        "speed_quadruple" => Action::SetSpeed(4.0),
        "speed_invalid" => Action::SetSpeed(f64::NAN),
        "volume_zero" => Action::Volume(0.0),
        "volume_half" => Action::Volume(50.0),
        "volume_full" => Action::Volume(100.0),
        "volume_invalid" => Action::Volume(101.0),
        "audio_track" => Action::AudioTrack(2),
        "audio_invalid" => Action::AudioTrack(0),
        "subtitles_off" => Action::SubtitleTrack(0),
        "subtitle_track" => Action::SubtitleTrack(2),
        "playlist" => Action::TogglePlaylist,
        "fullscreen" => Action::ToggleFullscreen,
        "stop" => Action::Stop,
        "quit" => Action::Quit,
        "next" => Action::Next,
        "previous" => Action::Previous,
        "select" => Action::Select(127),
        "select_invalid" => Action::Select(128),
        _ => unreachable!(),
    }
}

#[divan::bench(
    consts = [false, true],
    args = ["pause", "mute", "seek_backward", "seek_forward", "seek_nan", "scrub", "scrub_before_start", "scrub_past_end", "scrub_infinite", "speed_quarter", "speed_normal", "speed_double", "speed_quadruple", "speed_invalid", "volume_zero", "volume_half", "volume_full", "volume_invalid", "audio_track", "audio_invalid", "subtitles_off", "subtitle_track", "playlist", "fullscreen", "stop", "quit", "next", "previous", "select", "select_invalid"]
)]
fn dispatch<const LOADED: bool>(bencher: Bencher, name: &str) {
    let action = action(name);
    let initial = state(128, LOADED);
    assert_eq!(initial.loaded, LOADED);
    assert_eq!(initial.playlist.len(), 128);
    bencher
        .with_inputs(|| state(128, LOADED))
        .bench_local_refs(|state| black_box(state).dispatch(black_box(action.clone())));
}

#[divan::bench(args = [0, 1, 16, 256, 4096])]
fn open_playlist(bencher: Bencher, count: usize) {
    let mut check = UiState::default();
    let effects = check.dispatch(Action::Open(playlist(count)));
    assert_eq!(check.playlist.len(), count);
    assert_eq!(effects.len(), usize::from(count > 0));
    bencher
        .with_inputs(|| (UiState::default(), playlist(count)))
        .bench_local_refs(|(state, files)| {
            state.dispatch(Action::Open(black_box(std::mem::take(files))))
        });
}

#[divan::bench(consts = [0, 1, 2], args = [1, 16, 256, 4096])]
fn playlist_navigation<const OP: u8>(bencher: Bencher, count: usize) {
    let action = match OP {
        0 => Action::Next,
        1 => Action::Previous,
        2 => Action::Select(count - 1),
        _ => unreachable!(),
    };
    let mut check = state(count, true);
    let effects = check.dispatch(action.clone());
    assert_eq!(effects.len(), usize::from(count > 1 || OP == 2));
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, Effect::PlayPlaylistIndex(_)))
    );
    bencher
        .with_inputs(|| state(count, true))
        .bench_local_refs(|state| black_box(state).dispatch(black_box(action.clone())));
}

#[divan::bench(args = [10, 60, 120, 1000])]
fn scrub_sequence(bencher: Bencher, events: usize) {
    bencher
        .with_inputs(|| state(1, true))
        .bench_local_refs(|state| {
            for i in 0..events {
                black_box(state.dispatch(Action::SeekAbsolute(black_box((i * 37 % 7200) as f64))));
            }
            black_box(state.duration)
        });
}

#[divan::bench(args = [0.0, 120.0, 7200.0, -1.0, f64::NAN, f64::INFINITY])]
fn load_confirmation(bencher: Bencher, duration: f64) {
    let mut check = state(1, false);
    check.confirm_loaded(duration);
    assert!(check.loaded && check.duration.is_finite() && check.duration >= 0.0);
    bencher
        .with_inputs(|| state(1, false))
        .bench_local_refs(|state| {
            state.confirm_loaded(black_box(duration));
            black_box((state.loaded, state.duration))
        });
}

#[divan::bench(args = [" ", "p", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "f", "q", ">", "<", "Escape", "Enter", "m", "", "P", "Arrow", "ArrowLeftExtra", "字幕", "🦊"])]
fn shortcut_lookup(bencher: Bencher, key: &str) {
    bencher.bench_local(|| shortcut(black_box(key)));
}

#[divan::bench(
    consts = [false, true],
    args = ["frame-step", "frame-back-step", "secondary-subtitle", "rotation", "flip-vertical", "flip-horizontal", "replaygain", "deband", "equalizer"]
)]
fn optional_playback_controls<const LOADED: bool>(bencher: Bencher, name: &str) {
    let value = match name {
        "rotation" => 90.0,
        "secondary-subtitle" | "replaygain" => 2.0,
        "deband" | "equalizer" => 1.0,
        _ => 0.0,
    };
    let mut check = state(1, LOADED);
    let action = PlaybackControl::from_ui_action(name, value).unwrap();
    assert_eq!(
        check.dispatch_playback_control(action).len(),
        usize::from(LOADED)
    );
    bencher
        .with_inputs(|| state(1, LOADED))
        .bench_local_refs(|state| {
            state.dispatch_playback_control(black_box(
                PlaybackControl::from_ui_action(black_box(name), black_box(value)).unwrap(),
            ))
        });
}

#[divan::bench(consts = [16, 256, 4096], args = ["episode", "00012", "missing", ""])]
fn playlist_filter<const COUNT: usize>(bencher: Bencher, query: &str) {
    let queue = state(COUNT, true);
    bencher.bench(|| black_box(&queue).filtered_indices(black_box(query)));
}
