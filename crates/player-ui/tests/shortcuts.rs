use player_core::Command;
use player_ui::shortcut;

#[test]
fn keyboard_navigation_matches_documented_mpv_defaults() {
    for (key, command) in [
        (" ", Command::TogglePause),
        ("p", Command::TogglePause),
        ("ArrowLeft", Command::SeekRelative(-5.0)),
        ("ArrowRight", Command::SeekRelative(5.0)),
        ("ArrowUp", Command::SeekRelative(60.0)),
        ("ArrowDown", Command::SeekRelative(-60.0)),
        ("f", Command::ToggleFullscreen),
        ("q", Command::Quit),
        (">", Command::Next),
        ("<", Command::Previous),
    ] {
        assert_eq!(shortcut(key), Some(command));
    }
    assert_eq!(shortcut("unbound"), None);
}
