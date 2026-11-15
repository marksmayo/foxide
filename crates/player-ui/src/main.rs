use clap::Parser;
use player_render::{Engine, Snapshot};
use player_ui::{Action, Effect, UiState};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc, time::Duration};
// Only pinned Slint compiler output may use its generated vtable unsafe code.
// Hand-written UI glue remains under deny(unsafe_code); FFI lives in player-render.
#[allow(unsafe_code)]
mod generated {
    slint::include_modules!();
}
use generated::*;
mod preview;

#[derive(Parser)]
#[command(about = "Foxide desktop player — Slint and embedded libmpv")]
struct Args {
    /// Local media files or a stream URL.
    media: Vec<PathBuf>,
    /// Automatically close the window after this many seconds (UI diagnostics).
    #[arg(long, hide = true)]
    quit_after: Option<u64>,
    /// Render the empty Slint layout to PNG without opening a native window.
    #[arg(long)]
    preview_png: Option<PathBuf>,
}

fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}

fn dispatch(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    engine: &Option<Rc<Engine>>,
    action: Action,
) {
    let queue_changed = matches!(action, Action::Open(_));
    let mut state = state.borrow_mut();
    let effects = state.dispatch(action);
    if queue_changed {
        ui.set_queue(ModelRc::new(VecModel::from(
            state
                .playlist
                .iter()
                .map(|p| {
                    p.file_name()
                        .unwrap_or(p.as_os_str())
                        .to_string_lossy()
                        .into_owned()
                        .into()
                })
                .collect::<Vec<slint::SharedString>>(),
        )));
        ui.set_status("Opening media…".into());
    }
    ui.set_selected_row(state.selected as i32);
    ui.set_queue_visible(state.playlist_visible);
    ui.set_fullscreen_mode(state.fullscreen);
    ui.set_loaded(state.loaded);
    for effect in effects {
        let args: Vec<String> = match effect {
            Effect::Load(path) => vec![
                "loadfile".into(),
                player_core::external_path(path)
                    .to_string_lossy()
                    .into_owned(),
                "replace".into(),
            ],
            Effect::Pause(value) => vec![
                "set".into(),
                "pause".into(),
                if value { "yes" } else { "no" }.into(),
            ],
            Effect::Mute(value) => vec![
                "set".into(),
                "mute".into(),
                if value { "yes" } else { "no" }.into(),
            ],
            Effect::Seek(value) => vec!["seek".into(), value.to_string(), "relative+exact".into()],
            Effect::SeekAbsolute(value) => {
                vec!["seek".into(), value.to_string(), "absolute+exact".into()]
            }
            Effect::Speed(value) => vec!["set".into(), "speed".into(), value.to_string()],
            Effect::Volume(value) => vec!["set".into(), "volume".into(), value.to_string()],
            Effect::AudioTrack(value) => vec!["set".into(), "aid".into(), value.to_string()],
            Effect::SubtitleTrack(value) => vec![
                "set".into(),
                "sid".into(),
                if value == 0 {
                    "no".into()
                } else {
                    value.to_string()
                },
            ],
            Effect::Stop => vec!["stop".into()],
            Effect::Quit => {
                let _ = slint::quit_event_loop();
                continue;
            }
        };
        match engine {
            Some(engine) => {
                if let Err(error) = engine.command(args) {
                    ui.set_status(error.into());
                }
            }
            None => ui.set_status("Playback engine unavailable — see setup instructions".into()),
        }
    }
}

fn open_dialog(weak: slint::Weak<PlayerWindow>, subtitle: bool) {
    std::thread::spawn(move || {
        let paths = futures_lite::future::block_on(async {
            if subtitle {
                rfd::AsyncFileDialog::new()
                    .add_filter("Subtitles", &["srt", "ass", "ssa", "vtt"])
                    .pick_file()
                    .await
                    .map(|file| vec![file.path().to_path_buf()])
            } else {
                rfd::AsyncFileDialog::new()
                    .set_title("Open media files")
                    .pick_files()
                    .await
                    .map(|files| files.iter().map(|file| file.path().to_path_buf()).collect())
            }
        });
        if let Some(paths) = paths {
            let _ = weak.upgrade_in_event_loop(move |ui| {
                let paths: Option<Vec<slint::SharedString>> = paths
                    .iter()
                    .map(|path| path.to_str().map(Into::into))
                    .collect();
                if let Some(paths) = paths {
                    ui.invoke_files_picked(ModelRc::new(VecModel::from(paths)), subtitle);
                } else {
                    ui.set_status("This file path is not valid Unicode".into());
                }
            });
        }
    });
}

fn refresh(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    snapshot: &Snapshot,
    previous: &mut Snapshot,
) {
    if snapshot == previous {
        return;
    }
    let expected = state
        .borrow()
        .playlist
        .get(state.borrow().selected)
        .map(|p| {
            player_core::external_path(p.clone())
                .to_string_lossy()
                .into_owned()
        });
    if snapshot.loaded && expected.as_deref() == Some(snapshot.path.as_str()) {
        let mut state = state.borrow_mut();
        state.confirm_loaded(snapshot.duration);
        state.paused = snapshot.paused;
        state.muted = snapshot.muted;
        ui.set_loaded(true);
        ui.set_paused(snapshot.paused);
        if !previous.loaded || snapshot.title != previous.title {
            ui.set_media_title(snapshot.title.clone().into());
        }
        ui.set_position(snapshot.position as f32);
        ui.set_duration(snapshot.duration as f32);
        if !previous.loaded
            || snapshot.position.max(0.0) as u64 != previous.position.max(0.0) as u64
        {
            ui.set_time_label(clock(snapshot.position).into());
        }
        if !previous.loaded
            || snapshot.duration.max(0.0) as u64 != previous.duration.max(0.0) as u64
        {
            ui.set_duration_label(clock(snapshot.duration).into());
        }
        ui.set_volume(snapshot.volume as f32);
        ui.set_muted(snapshot.muted);
        ui.set_status(
            format!(
                "{} · {:.2}×",
                if snapshot.paused { "Paused" } else { "Playing" },
                snapshot.speed
            )
            .into(),
        );
    }
    if !snapshot.error.is_empty() {
        ui.set_status(snapshot.error.clone().into());
        ui.set_loaded(false);
        state.borrow_mut().loaded = false;
    }
    if snapshot.tracks != previous.tracks {
        let model = |kind| {
            ModelRc::new(VecModel::from(
                snapshot
                    .tracks
                    .iter()
                    .filter(|t| t.kind == kind)
                    .map(|t| TrackRow {
                        id: t.id as i32,
                        label: t.label.clone().into(),
                        selected: t.selected,
                    })
                    .collect::<Vec<_>>(),
            ))
        };
        ui.set_audio_tracks(model("audio"));
        ui.set_subtitle_tracks(model("sub"));
    }
    if snapshot.hwdec != previous.hwdec
        || snapshot.error != previous.error
        || ui.get_diagnostics() == "No media loaded"
    {
        ui.set_diagnostics(format!("Video renderer: shared OpenGL texture\nRequested decoder: auto-safe\nActive decoder: {}\n\nHardware decoding does not by itself certify zero-copy.\nNative GPU/power acceptance remains platform-specific.\n\n{}", if snapshot.hwdec.is_empty() { "not active" } else { &snapshot.hwdec }, snapshot.error).into());
    }
    *previous = snapshot.clone();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(path) = args.preview_png {
        return preview::render(&path);
    }
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let ui = PlayerWindow::new()?;
    let state = Rc::new(RefCell::new(UiState::default()));
    let engine = match Engine::new() {
        Ok(engine) => Some(Rc::new(engine)),
        Err(error) => {
            ui.set_status(error.clone().into());
            ui.set_diagnostics(error.into());
            None
        }
    };
    if let Some(engine) = &engine {
        let weak_size = ui.as_weak();
        let weak_frame = ui.as_weak();
        let weak_error = ui.as_weak();
        let weak_wake = ui.as_weak();
        player_render::attach(
            ui.window(),
            engine,
            move || {
                weak_size
                    .upgrade()
                    .map(|ui| {
                        let scale = ui.window().scale_factor();
                        (
                            (ui.get_video_width() * scale).round() as u32,
                            (ui.get_video_height() * scale).round() as u32,
                        )
                    })
                    .unwrap_or((1, 1))
            },
            move |frame| {
                if let Some(ui) = weak_frame.upgrade() {
                    ui.set_video_frame(frame);
                }
            },
            move |error| {
                if let Some(ui) = weak_error.upgrade() {
                    ui.set_status(error.into());
                }
            },
            Arc::new(move || {
                let weak = weak_wake.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.window().request_redraw();
                    }
                });
            }),
        )?;
    }
    let (weak, controller, playback) = (ui.as_weak(), state.clone(), engine.clone());
    ui.on_action(move |name, value| {
        let Some(ui) = weak.upgrade() else { return };
        let action = match name.as_str() {
            "open" | "subtitle-file" => {
                open_dialog(weak.clone(), name == "subtitle-file");
                return;
            }
            "pause" => Action::TogglePause,
            "stop" => Action::Stop,
            "next" => Action::Next,
            "previous" => Action::Previous,
            "seek-relative" => Action::Seek(value as f64),
            "seek" => Action::SeekAbsolute(value as f64),
            "speed" => Action::SetSpeed(value as f64),
            "volume" => Action::Volume(value as f64),
            "select" if value >= 0.0 => Action::Select(value as usize),
            "audio-track" if value > 0.0 => Action::AudioTrack(value as u32),
            "subtitle-track" if value >= 0.0 => Action::SubtitleTrack(value as u32),
            "playlist" => Action::TogglePlaylist,
            "fullscreen" => Action::ToggleFullscreen,
            "quit" => Action::Quit,
            "mute" => Action::ToggleMute,
            _ => return,
        };
        dispatch(&ui, &controller, &playback, action);
    });
    let (weak, controller, playback) = (ui.as_weak(), state.clone(), engine.clone());
    ui.on_open_url(move |url| {
        if let Some(ui) = weak.upgrade() {
            if ["https://", "http://", "rtsp://"]
                .iter()
                .any(|prefix| url.starts_with(prefix))
            {
                dispatch(
                    &ui,
                    &controller,
                    &playback,
                    Action::Open(vec![PathBuf::from(url.as_str())]),
                );
            } else {
                ui.set_status("Enter an HTTP, HTTPS or RTSP URL".into());
            }
        }
    });
    let (weak, controller, playback) = (ui.as_weak(), state.clone(), engine.clone());
    ui.on_files_picked(move |files, subtitle| {
        use slint::Model;
        if let Some(ui) = weak.upgrade() {
            let paths: Vec<PathBuf> = files.iter().map(|s| PathBuf::from(s.as_str())).collect();
            if subtitle {
                if let (Some(path), Some(engine)) = (paths.first(), playback.as_ref())
                    && let Err(error) = engine.command(vec![
                        "sub-add".into(),
                        player_core::external_path(path.clone())
                            .to_string_lossy()
                            .into_owned(),
                        "select".into(),
                    ])
                {
                    ui.set_status(error.into());
                }
            } else {
                dispatch(&ui, &controller, &playback, Action::Open(paths));
            }
        }
    });
    if !args.media.is_empty() {
        dispatch(&ui, &state, &engine, Action::Open(args.media));
    }
    let timer = slint::Timer::default();
    if let Some(playback) = engine.clone() {
        let weak = ui.as_weak();
        let controller = state.clone();
        let mut previous = Snapshot::default();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(200),
            move || {
                if let (Some(ui), Some(snapshot)) = (weak.upgrade(), playback.snapshot()) {
                    refresh(&ui, &controller, &snapshot, &mut previous);
                }
            },
        );
    }
    let exit_timer = slint::Timer::default();
    if let Some(seconds) = args.quit_after {
        exit_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_secs(seconds),
            || {
                let _ = slint::quit_event_loop();
            },
        );
    }
    ui.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::{
        Model,
        platform::{
            Platform, WindowAdapter,
            software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        },
    };

    struct TestPlatform;
    impl Platform for TestPlatform {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer))
        }
    }

    #[test]
    fn playback_refresh_preserves_visible_state_across_ticks_tracks_and_errors() {
        slint::platform::set_platform(Box::new(TestPlatform)).unwrap();
        let ui = PlayerWindow::new().unwrap();
        let state = Rc::new(RefCell::new(UiState::default()));
        state
            .borrow_mut()
            .dispatch(Action::Open(vec!["movie.mkv".into()]));
        let mut previous = Snapshot::default();
        let mut snapshot = Snapshot {
            loaded: true,
            path: "movie.mkv".into(),
            title: "Movie".into(),
            position: 65.1,
            duration: 125.0,
            speed: 1.0,
            volume: 75.0,
            hwdec: String::new(),
            tracks: vec![player_render::Track {
                id: 1,
                kind: "audio".into(),
                label: "English".into(),
                selected: true,
            }],
            ..Default::default()
        };
        refresh(&ui, &state, &snapshot, &mut previous);
        assert!(ui.get_loaded());
        assert_eq!(ui.get_media_title(), "Movie");
        assert_eq!(ui.get_time_label(), "01:05");
        assert_eq!(ui.get_duration_label(), "02:05");
        assert_eq!(ui.get_status(), "Playing · 1.00×");
        assert_eq!(ui.get_audio_tracks().row_count(), 1);
        let diagnostics = ui.get_diagnostics();
        assert!(diagnostics.contains("Active decoder: not active"));
        snapshot.position = 65.9;
        refresh(&ui, &state, &snapshot, &mut previous);
        assert_eq!(ui.get_position(), 65.9);
        assert_eq!(ui.get_time_label(), "01:05");
        assert_eq!(ui.get_diagnostics(), diagnostics);
        snapshot.position = 66.0;
        snapshot.paused = true;
        snapshot.speed = 1.5;
        snapshot.title = "New title".into();
        snapshot.duration = 126.0;
        snapshot.hwdec = "d3d11va".into();
        snapshot.tracks[0].selected = false;
        refresh(&ui, &state, &snapshot, &mut previous);
        assert_eq!(ui.get_time_label(), "01:06");
        assert_eq!(ui.get_duration_label(), "02:06");
        assert_eq!(ui.get_status(), "Paused · 1.50×");
        assert_eq!(ui.get_media_title(), "New title");
        assert!(ui.get_diagnostics().contains("d3d11va"));
        assert!(!ui.get_audio_tracks().row_data(0).unwrap().selected);
        snapshot.error = "Decode failed".into();
        refresh(&ui, &state, &snapshot, &mut previous);
        assert!(!ui.get_loaded());
        assert_eq!(ui.get_status(), "Decode failed");
        assert!(ui.get_diagnostics().contains("Decode failed"));
    }
}
