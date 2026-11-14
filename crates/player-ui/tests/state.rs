use player_ui::{Action, Effect, UiState};

#[test]
fn mute_uses_confirmed_state_and_never_assumes_command_success() {
    let mut state = UiState::default();
    assert_eq!(state.dispatch(Action::ToggleMute), [Effect::Mute(true)]);
    assert!(!state.muted);
    state.muted = true;
    assert_eq!(state.dispatch(Action::ToggleMute), [Effect::Mute(false)]);
}

#[test]
fn empty_player_cannot_seek_and_opening_files_populates_one_ordered_queue() {
    let mut state = UiState::default();
    assert!(state.dispatch(Action::Seek(30.0)).is_empty());
    assert_eq!(
        state.dispatch(Action::Open(vec!["one.mkv".into(), "two.mkv".into()])),
        [Effect::Load("one.mkv".into())]
    );
    assert_eq!(state.playlist.len(), 2);
    assert!(
        !state.loaded,
        "only the engine can confirm successful playback"
    );
}

#[test]
fn playback_actions_wait_for_engine_confirmation_and_validate_values() {
    let mut state = UiState::default();
    assert!(state.dispatch(Action::TogglePause).is_empty());
    state.confirm_loaded(120.0);
    assert_eq!(state.dispatch(Action::TogglePause), [Effect::Pause(true)]);
    assert!(
        !state.paused,
        "do not show optimistic success before acknowledgement"
    );
    state.paused = true;
    assert_eq!(state.dispatch(Action::TogglePause), [Effect::Pause(false)]);
    assert_eq!(state.dispatch(Action::Seek(30.0)), [Effect::Seek(30.0)]);
    assert!(state.dispatch(Action::Seek(f64::NAN)).is_empty());
    assert!(state.dispatch(Action::SetSpeed(20.0)).is_empty());
    assert_eq!(state.dispatch(Action::SetSpeed(2.0)), [Effect::Speed(2.0)]);
}

#[test]
fn queue_navigation_tracks_and_window_commands_share_the_same_dispatch() {
    let mut state = UiState::default();
    state.dispatch(Action::Open(vec!["one.mkv".into(), "two.mkv".into()]));
    state.confirm_loaded(120.0);
    assert_eq!(
        state.dispatch(Action::Next),
        [Effect::Load("two.mkv".into())]
    );
    assert!(!state.loaded);
    assert!(state.dispatch(Action::Next).is_empty());
    assert_eq!(
        state.dispatch(Action::Previous),
        [Effect::Load("one.mkv".into())]
    );
    state.confirm_loaded(120.0);
    assert_eq!(
        state.dispatch(Action::SeekAbsolute(150.0)),
        [Effect::SeekAbsolute(120.0)]
    );
    assert_eq!(state.dispatch(Action::Volume(50.0)), [Effect::Volume(50.0)]);
    assert!(state.dispatch(Action::Volume(-1.0)).is_empty());
    assert_eq!(
        state.dispatch(Action::AudioTrack(2)),
        [Effect::AudioTrack(2)]
    );
    assert_eq!(
        state.dispatch(Action::SubtitleTrack(0)),
        [Effect::SubtitleTrack(0)]
    );
    assert!(state.dispatch(Action::AudioTrack(0)).is_empty());
    state.dispatch(Action::TogglePlaylist);
    assert!(state.playlist_visible);
    state.dispatch(Action::ToggleFullscreen);
    assert!(state.fullscreen);
    assert_eq!(state.dispatch(Action::Stop), [Effect::Stop]);
    assert!(!state.loaded);
    assert_eq!(state.dispatch(Action::Quit), [Effect::Quit]);
}
