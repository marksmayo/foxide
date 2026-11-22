//! Settings CPU benchmarks, separate from playback policy code generation.
use divan::{Bencher, black_box};
fn main() {
    divan::main();
}

#[divan::bench(args = ["Space", "p", "ArrowLeft", "ArrowRight", "F11", "m", "q", "unbound"])]
fn configured_shortcut(bencher: Bencher, key: &str) {
    let preferences = player_settings::Preferences::default();
    bencher
        .bench(|| black_box(&preferences).resolve_key(black_box(key), false, false, false, false));
}

#[divan::bench(consts = [false, true])]
fn preference_round_trip<const CUSTOM: bool>(bencher: Bencher) {
    let mut preferences = player_settings::Preferences::default();
    if CUSTOM {
        preferences.bind("Ctrl+K", "pause").unwrap();
        preferences.set_appearance(true, "#3178ce").unwrap();
        preferences.set_compact(true);
    }
    let json = preferences.to_json();
    assert_eq!(
        player_settings::Preferences::from_json(&json).unwrap(),
        preferences
    );
    bencher.bench(|| player_settings::Preferences::from_json(black_box(&json)).unwrap());
}

#[divan::bench(consts = [false, true])]
fn serialize_preferences<const COMPACT: bool>(bencher: Bencher) {
    let mut preferences = player_settings::Preferences::default();
    preferences.set_compact(COMPACT);
    bencher.bench(|| black_box(&preferences).to_json());
}

#[divan::bench(args = ["Ctrl+K", "Alt+F12", "Meta+Shift+X", "Escape"])]
fn edit_shortcut(bencher: Bencher, key: &str) {
    bencher
        .with_inputs(player_settings::Preferences::default)
        .bench_local_refs(|preferences| black_box(preferences).bind(black_box(key), "pause"));
}
