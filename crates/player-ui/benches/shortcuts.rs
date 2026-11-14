//! CPU benchmarks for the keyboard shortcut policy.
use divan::black_box;
use player_ui::shortcut;

fn main() {
    divan::main();
}

const KEYS: &[&str] = &[
    " ",
    "p",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "ArrowDown",
    "f",
    "q",
    ">",
    "<",
    "Escape",
    "Enter",
    "m",
];

#[divan::bench]
fn shortcut_lookup_all_keys() -> usize {
    KEYS.iter()
        .filter_map(|key| shortcut(black_box(key)))
        .count()
}

#[divan::bench]
fn shortcut_lookup_unmapped() -> Option<player_core::Command> {
    shortcut(black_box("Escape"))
}
