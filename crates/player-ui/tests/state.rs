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

#[test]
fn frame_controls_are_available_only_for_loaded_media() {
    let mut state = UiState::default();
    let step = Action::control("frame-step", 0.0).unwrap();
    assert!(state.dispatch(step.clone()).is_empty());
    state.confirm_loaded(120.0);
    assert_eq!(
        state.dispatch(step),
        [Effect::Engine(vec!["frame-step".into()])]
    );
    assert_eq!(
        state.dispatch(Action::control("frame-back-step", 0.0).unwrap()),
        [Effect::Engine(vec!["frame-back-step".into()])]
    );
    assert!(Action::control("unknown", 0.0).is_none());
}

#[test]
fn secondary_subtitles_select_a_second_track_or_turn_it_off() {
    let mut state = UiState::default();
    state.confirm_loaded(100.0);
    assert_eq!(
        state.dispatch(Action::control("secondary-subtitle", 2.0).unwrap()),
        [Effect::Engine(vec![
            "set".into(),
            "secondary-sid".into(),
            "2".into()
        ])]
    );
    assert_eq!(
        state.dispatch(Action::control("secondary-subtitle", 0.0).unwrap()),
        [Effect::Engine(vec![
            "set".into(),
            "secondary-sid".into(),
            "no".into()
        ])]
    );
    for invalid in [-1.0, 1.5, f64::NAN, f64::INFINITY, u32::MAX as f64 + 1.0] {
        assert!(Action::control("secondary-subtitle", invalid).is_none());
    }
}

#[test]
fn rotation_and_flip_controls_preserve_explicit_opt_in_and_reject_invalid_angles() {
    let mut state = UiState::default();
    state.confirm_loaded(100.0);
    for angle in [0.0, 90.0, 180.0, 270.0] {
        assert_eq!(
            state.dispatch(Action::control("rotation", angle).unwrap()),
            [Effect::Engine(vec![
                "set".into(),
                "video-rotate".into(),
                (angle as u16).to_string()
            ])]
        );
    }
    for invalid in [45.0, 360.0, -90.0, f64::NAN] {
        assert!(Action::control("rotation", invalid).is_none());
    }
    assert_eq!(
        state.dispatch(Action::control("flip-vertical", 0.0).unwrap()),
        [Effect::Engine(vec![
            "vf".into(),
            "toggle".into(),
            "@foxide-vflip:vflip".into()
        ])]
    );
    assert_eq!(
        state.dispatch(Action::control("flip-horizontal", 0.0).unwrap()),
        [Effect::Engine(vec![
            "vf".into(),
            "toggle".into(),
            "@foxide-hflip:hflip".into()
        ])]
    );
}

#[test]
fn replaygain_is_explicit_with_off_track_and_album_modes() {
    let mut state = UiState::default();
    state.confirm_loaded(100.0);
    for (value, mode) in [(0.0, "no"), (1.0, "track"), (2.0, "album")] {
        assert_eq!(
            state.dispatch(Action::control("replaygain", value).unwrap()),
            [Effect::Engine(vec![
                "set".into(),
                "replaygain".into(),
                mode.into()
            ])]
        );
    }
    for value in [-1.0, 3.0, 1.5, f64::NAN] {
        assert!(Action::control("replaygain", value).is_none());
    }
}

#[test]
fn debanding_can_be_enabled_and_disabled_without_changing_startup_defaults() {
    let mut state = UiState::default();
    state.confirm_loaded(100.0);
    for (value, mode) in [(0.0, "no"), (1.0, "yes")] {
        assert_eq!(
            state.dispatch(Action::control("deband", value).unwrap()),
            [Effect::Engine(vec![
                "set".into(),
                "deband".into(),
                mode.into()
            ])]
        );
    }
    assert!(Action::control("deband", 0.5).is_none());
}

#[test]
fn equalizer_presets_are_bounded_and_can_be_reset() {
    let mut state = UiState::default();
    state.confirm_loaded(100.0);
    for (value, filter) in [
        (0.0, ""),
        (1.0, "lavfi=[equalizer=f=100:t=o:w=2:g=6]"),
        (2.0, "lavfi=[equalizer=f=3000:t=o:w=2:g=4]"),
    ] {
        assert_eq!(
            state.dispatch(Action::control("equalizer", value).unwrap()),
            [Effect::Engine(vec![
                "af".into(),
                "set".into(),
                filter.into()
            ])]
        );
    }
    for value in [-1.0, 3.0, 1.5, f64::NAN] {
        assert!(Action::control("equalizer", value).is_none());
    }
}

#[test]
fn playlist_search_preserves_original_selection_indices_and_ignores_folder_names() {
    let mut state = UiState::default();
    state.dispatch(Action::Open(vec![
        "/media/Épisode 01.mkv".into(),
        "/media/Other.flac".into(),
        "/media/ÉPISODE 02.mkv".into(),
    ]));
    assert_eq!(state.filtered_indices("épisode"), [0, 2]);
    assert_eq!(state.filtered_indices("  OTHER "), [1]);
    assert_eq!(state.filtered_indices("media"), Vec::<usize>::new());
    assert_eq!(state.filtered_indices(""), [0, 1, 2]);
    assert_eq!(
        state.dispatch(Action::Select(state.filtered_indices("02")[0])),
        [Effect::Load("/media/ÉPISODE 02.mkv".into())]
    );
    assert_eq!(state.selected, 2);
}
