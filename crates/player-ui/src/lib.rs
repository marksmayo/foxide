//! Keyboard policy for the future Foxide OSD. This crate does not create a window yet.
use player_core::Command;

/// Initial mpv-compatible shortcut subset. Modifiers and text editing are handled by the UI.
pub fn shortcut(key: &str) -> Option<Command> {
    Some(match key {
        " " | "p" => Command::TogglePause,
        "ArrowLeft" => Command::SeekRelative(-5.0),
        "ArrowRight" => Command::SeekRelative(5.0),
        "ArrowUp" => Command::SeekRelative(60.0),
        "ArrowDown" => Command::SeekRelative(-60.0),
        "f" => Command::ToggleFullscreen,
        "q" => Command::Quit,
        ">" => Command::Next,
        "<" => Command::Previous,
        _ => return None,
    })
}
