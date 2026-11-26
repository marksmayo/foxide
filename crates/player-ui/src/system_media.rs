use super::PlayerWindow;
use player_render::Snapshot;
use player_ui::{
    Action, UiState,
    system_media_controls::{MediaCommand, action_for_command},
};
use playwire::{Capabilities, MediaControls, PlaybackState, PlayerConfig, Track};
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

const ATTACH_RETRY_INTERVAL: Duration = Duration::from_secs(1);

pub(super) enum AttachOutcome {
    Attached(SystemMediaSession),
    RetryableNotReady,
    RetryableFailure(String),
    PermanentFailure(String),
}

enum AttachFailure {
    Retryable(String),
    Permanent(String),
}

fn classify_attach_error(error: playwire::Error) -> AttachFailure {
    match error {
        playwire::Error::Unavailable(message) => AttachFailure::Retryable(message),
        error => AttachFailure::Permanent(error.to_string()),
    }
}

pub(super) struct AttachRetry {
    next_attempt: Instant,
    finished: bool,
}

impl Default for AttachRetry {
    fn default() -> Self {
        Self {
            next_attempt: Instant::now(),
            finished: false,
        }
    }
}

impl AttachRetry {
    pub(super) fn due(&mut self, now: Instant) -> bool {
        if self.finished || now < self.next_attempt {
            return false;
        }
        self.next_attempt = now + ATTACH_RETRY_INTERVAL;
        true
    }

    pub(super) fn finish(&mut self) {
        self.finished = true;
    }
}

fn drain_commands(receiver: &Receiver<MediaCommand>, mut apply: impl FnMut(Action)) {
    while let Ok(command) = receiver.try_recv() {
        if let Some(action) = action_for_command(command) {
            apply(action);
        }
    }
}

pub(super) struct SystemMediaSession {
    controls: MediaControls,
    identity: TrackIdentity,
    pub(super) last_error: Option<String>,
    _command_timer: slint::Timer,
}

#[derive(Default)]
struct TrackIdentity {
    active: bool,
    source_path: String,
    sequence: u64,
}

impl TrackIdentity {
    fn id_for(&mut self, snapshot: &Snapshot, state: &UiState) -> String {
        let active = snapshot.loaded && state.loaded && snapshot.error.is_empty();
        if !active {
            self.active = false;
            return String::new();
        }
        if !self.active || self.source_path != snapshot.path {
            self.sequence = self.sequence.wrapping_add(1).max(1);
            self.source_path.clone_from(&snapshot.path);
        }
        self.active = true;
        format!("foxide_track_{}", self.sequence)
    }
}

impl SystemMediaSession {
    pub(super) fn publish(&mut self, snapshot: &Snapshot, state: &UiState) -> Result<(), String> {
        let track_id = self.identity.id_for(snapshot, state);
        self.controls
            .set_state(&playback_state(snapshot, state, &track_id))
            .map_err(|error| error.to_string())
    }
}

pub(super) fn attach(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    engine: &Option<Rc<player_render::Engine>>,
) -> Result<AttachOutcome, String> {
    let Some(config) = platform_config(ui)? else {
        return Ok(AttachOutcome::RetryableNotReady);
    };
    let (sender, receiver) = mpsc::channel();
    let controls = match MediaControls::new(config, move |event| {
        let Some(command) = command_for_event(event) else {
            return;
        };
        let _ = sender.send(command);
    }) {
        Ok(controls) => controls,
        Err(error) => {
            return Ok(match classify_attach_error(error) {
                AttachFailure::Retryable(message) => AttachOutcome::RetryableFailure(message),
                AttachFailure::Permanent(message) => AttachOutcome::PermanentFailure(message),
            });
        }
    };
    let weak = ui.as_weak();
    let state = state.clone();
    let engine = engine.clone();
    let command_timer = slint::Timer::default();
    command_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            drain_commands(&receiver, |action| {
                if let Some(ui) = weak.upgrade() {
                    super::dispatch(&ui, &state, &engine, action);
                }
            });
        },
    );
    Ok(AttachOutcome::Attached(SystemMediaSession {
        controls,
        identity: TrackIdentity::default(),
        last_error: None,
        _command_timer: command_timer,
    }))
}

fn command_for_event(event: playwire::Event) -> Option<MediaCommand> {
    match event {
        playwire::Event::Play => Some(MediaCommand::Play),
        playwire::Event::Pause => Some(MediaCommand::Pause),
        playwire::Event::PlayPause => Some(MediaCommand::Toggle),
        playwire::Event::Stop => Some(MediaCommand::Stop),
        playwire::Event::Next => Some(MediaCommand::Next),
        playwire::Event::Previous => Some(MediaCommand::Previous),
        playwire::Event::SeekTo(position) => Some(MediaCommand::SeekTo(position.as_secs_f64())),
        playwire::Event::SeekBy(offset) => Some(MediaCommand::SeekBy(offset)),
        _ => None,
    }
}

fn playback_state(snapshot: &Snapshot, state: &UiState, track_id: &str) -> PlaybackState {
    let loaded = snapshot.loaded && state.loaded && snapshot.error.is_empty();
    let duration = loaded
        .then(|| seconds_to_duration(snapshot.duration))
        .flatten()
        .filter(|duration| !duration.is_zero());
    let position = if loaded {
        seconds_to_duration(snapshot.position)
            .unwrap_or_default()
            .min(duration.unwrap_or(Duration::MAX))
    } else {
        Duration::ZERO
    };
    let track = loaded.then(|| Track {
        id: track_id.to_owned(),
        title: if snapshot.title.is_empty() {
            "Unknown media".into()
        } else {
            snapshot.title.clone()
        },
        artists: if snapshot.artist.is_empty() {
            Vec::new()
        } else {
            vec![snapshot.artist.clone()]
        },
        album: snapshot.album.clone(),
        // Do not publish local paths or private media locations to the OS shell.
        artwork_url: String::new(),
        url: String::new(),
    });
    PlaybackState {
        track,
        playing: loaded && !snapshot.paused,
        position,
        duration,
        capabilities: Capabilities {
            can_go_next: loaded && state.selected + 1 < state.playlist.len(),
            can_go_previous: loaded && state.selected > 0,
            can_seek: loaded && duration.is_some(),
        },
        ..PlaybackState::default()
    }
}

fn seconds_to_duration(seconds: f64) -> Option<Duration> {
    (seconds.is_finite() && seconds >= 0.0)
        .then(|| Duration::try_from_secs_f64(seconds).ok())
        .flatten()
}

fn platform_config(ui: &PlayerWindow) -> Result<Option<PlayerConfig>, String> {
    let config = PlayerConfig::new("Foxide");
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let slint_handle = ui.window().window_handle();
        let Ok(handle) = slint_handle.window_handle() else {
            return Ok(None);
        };
        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get() as u64,
            _ => return Ok(None),
        };
        Ok(Some(config.hwnd(hwnd)))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = ui;
        Ok(Some(config))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttachRetry, MediaCommand, TrackIdentity, command_for_event, drain_commands, playback_state,
    };
    use player_render::Snapshot;
    use player_ui::{Action, UiState};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    #[test]
    fn unavailable_media_service_is_retryable_but_invalid_configuration_is_not() {
        assert!(matches!(
            super::classify_attach_error(playwire::Error::Unavailable("session bus not ready".into())),
            super::AttachFailure::Retryable(_)
        ));
        assert!(matches!(
            super::classify_attach_error(playwire::Error::Config("missing window handle".into())),
            super::AttachFailure::Permanent(_)
        ));
        assert!(matches!(
            super::classify_attach_error(playwire::Error::Unsupported),
            super::AttachFailure::Permanent(_)
        ));
    }

    #[test]
    fn retryable_window_not_ready_state_is_rate_limited() {
        let mut retry = AttachRetry::default();
        let start = Instant::now();

        assert!(retry.due(start));
        assert!(!retry.due(start + Duration::from_millis(50)));
        assert!(!retry.due(start + Duration::from_millis(999)));
        assert!(retry.due(start + Duration::from_secs(1)));
    }

    #[test]
    fn idle_session_can_attach_and_success_disables_future_retries() {
        let mut retry = AttachRetry::default();
        let start = Instant::now();

        // Session readiness is independent of whether a media track has loaded.
        assert!(retry.due(start));
        retry.finish();
        assert!(!retry.due(start + Duration::from_secs(60)));
    }

    #[test]
    fn publishes_current_track_metadata_and_transport_capabilities() {
        let mut state = UiState {
            playlist: vec![
                PathBuf::from("first.mp3"),
                PathBuf::from("current.mp3"),
                PathBuf::from("next.mp3"),
            ],
            selected: 1,
            ..UiState::default()
        };
        state.confirm_loaded(90.0);
        let snapshot = Snapshot {
            path: "C:/private/current.mp3".into(),
            title: "The Current Song".into(),
            artist: "Fox Artist".into(),
            album: "Fox Album".into(),
            loaded: true,
            paused: false,
            position: 13.5,
            duration: 90.0,
            ..Snapshot::default()
        };

        let published = playback_state(&snapshot, &state, "foxide_track_2");
        let track = published
            .track
            .expect("loaded media should publish a track");
        assert_eq!(track.id, "foxide_track_2");
        assert_eq!(track.title, "The Current Song");
        assert_eq!(track.artists, ["Fox Artist"]);
        assert_eq!(track.album, "Fox Album");
        assert!(track.url.is_empty(), "local media paths stay private");
        assert!(published.playing);
        assert_eq!(published.position, Duration::from_millis(13_500));
        assert_eq!(published.duration, Some(Duration::from_secs(90)));
        assert!(published.capabilities.can_go_previous);
        assert!(published.capabilities.can_go_next);
        assert!(published.capabilities.can_seek);
    }

    #[test]
    fn stopped_playback_clears_metadata_and_disables_transport() {
        let state = UiState::default();
        let snapshot = Snapshot {
            title: "Old title".into(),
            artist: "Old artist".into(),
            loaded: false,
            ..Snapshot::default()
        };

        let published = playback_state(&snapshot, &state, "foxide_track_1");
        assert!(published.track.is_none());
        assert!(!published.playing);
        assert_eq!(published.position, Duration::ZERO);
        assert_eq!(published.duration, None);
        assert!(!published.capabilities.can_go_next);
        assert!(!published.capabilities.can_go_previous);
        assert!(!published.capabilities.can_seek);
    }

    #[test]
    fn live_streams_do_not_publish_a_seek_bar() {
        let mut state = UiState::default();
        state.confirm_loaded(0.0);
        let snapshot = Snapshot {
            title: "Live stream".into(),
            loaded: true,
            position: f64::INFINITY,
            duration: f64::NAN,
            ..Snapshot::default()
        };

        let published = playback_state(&snapshot, &state, "foxide_track_1");
        assert!(published.track.is_some());
        assert_eq!(published.duration, None);
        assert_eq!(published.position, Duration::ZERO);
        assert!(!published.capabilities.can_seek);
    }

    #[test]
    fn media_events_map_to_supported_player_commands() {
        assert_eq!(
            command_for_event(playwire::Event::Play),
            Some(MediaCommand::Play)
        );
        assert_eq!(
            command_for_event(playwire::Event::Pause),
            Some(MediaCommand::Pause)
        );
        assert_eq!(
            command_for_event(playwire::Event::PlayPause),
            Some(MediaCommand::Toggle)
        );
        assert_eq!(
            command_for_event(playwire::Event::Next),
            Some(MediaCommand::Next)
        );
        assert_eq!(
            command_for_event(playwire::Event::Previous),
            Some(MediaCommand::Previous)
        );
        assert_eq!(
            command_for_event(playwire::Event::SeekTo(Duration::from_secs(8))),
            Some(MediaCommand::SeekTo(8.0))
        );
        assert_eq!(
            command_for_event(playwire::Event::SeekBy(-10.0)),
            Some(MediaCommand::SeekBy(-10.0))
        );
        assert_eq!(command_for_event(playwire::Event::SetVolume(0.5)), None);
    }

    #[test]
    fn track_ids_change_per_loaded_item_without_exposing_its_path() {
        let mut identity = TrackIdentity::default();
        let mut state = UiState::default();
        state.confirm_loaded(30.0);
        let first = Snapshot {
            path: "C:/private/first.mp3".into(),
            loaded: true,
            ..Snapshot::default()
        };
        let first_id = identity.id_for(&first, &state);
        assert_eq!(first_id, "foxide_track_1");
        assert_eq!(identity.id_for(&first, &state), first_id);

        let second = Snapshot {
            path: "C:/private/second.mp3".into(),
            loaded: true,
            ..Snapshot::default()
        };
        assert_eq!(identity.id_for(&second, &state), "foxide_track_2");
        assert!(!identity.id_for(&second, &state).contains("private"));
    }

    #[test]
    fn queued_system_commands_reach_the_shared_playback_actions() {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(MediaCommand::Play).unwrap();
        sender.send(MediaCommand::Next).unwrap();
        sender.send(MediaCommand::SeekTo(5.0)).unwrap();
        drop(sender);
        let mut actions = Vec::new();

        drain_commands(&receiver, |action| actions.push(action));

        assert_eq!(
            actions,
            vec![
                Action::SetPaused(false),
                Action::Next,
                Action::SeekAbsolute(5.0)
            ]
        );
    }
}
