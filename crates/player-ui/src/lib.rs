//! Shared command/state policy for the Foxide desktop and keyboard controls.
#![forbid(unsafe_code)]
use player_core::Command;
mod state;
pub mod system_media_controls;
pub use state::{Action, Effect, PlaybackControl, UiState, VideoControl};

/// Initial mpv-compatible shortcut subset. Modifiers and text editing are handled by the UI.
#[inline]
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

#[cfg(all(test, feature = "native-controls"))]
mod native_control_tests;
