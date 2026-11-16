use clap::Parser;
use player_render::{Engine, Snapshot};
use player_ui::{Action, Effect, Preferences, UiState};
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
    /// Read and save appearance and keyboard preferences at this path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Automatically close the window after this many seconds (UI diagnostics).
    #[arg(long, hide = true)]
    quit_after: Option<u64>,
    /// Render the empty Slint layout to PNG without opening a native window.
    #[arg(long)]
    preview_png: Option<PathBuf>,
    /// Explicit diagnostic script; normal startup never loads scripts.
    #[arg(long, hide = true)]
    benchmark_script: Option<PathBuf>,
    #[arg(long, default_value_t = 1.0)]
    speed: f64,
    #[arg(long)]
    paused: bool,
}

fn startup_commands(args: &Args) -> Result<Vec<Vec<String>>, String> {
    if !args.speed.is_finite() || !(0.25..=4.0).contains(&args.speed) {
        return Err("Speed must be between 0.25 and 4".into());
    }
    let mut commands = Vec::new();
    if let Some(path) = &args.benchmark_script {
        let path = player_core::external_path(path.clone());
        commands.push(vec![
            "load-script".into(),
            path.to_str().ok_or("Script path must be Unicode")?.into(),
        ]);
    }
    if args.speed != 1.0 {
        commands.push(vec!["set".into(), "speed".into(), args.speed.to_string()]);
    }
    if args.paused {
        commands.push(vec!["set".into(), "pause".into(), "yes".into()]);
    }
    Ok(commands)
}

fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}

fn update_queue(ui: &PlayerWindow, state: &UiState) {
    let rows = state
        .filtered_indices(ui.get_queue_filter().as_str())
        .into_iter()
        .map(|index| {
            let path = &state.playlist[index];
            QueueRow {
                source_index: index as i32,
                label: path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned()
                    .into(),
            }
        })
        .collect::<Vec<_>>();
    ui.set_queue_count(state.playlist.len() as i32);
    ui.set_queue(ModelRc::new(VecModel::from(rows)));
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
        ui.set_queue_filter("".into());
        update_queue(ui, &state);
        ui.set_status("Opening media…".into());
    }
    ui.set_selected_row(state.selected as i32);
    ui.set_queue_visible(state.playlist_visible);
    ui.set_fullscreen_mode(state.fullscreen);
    ui.set_loaded(state.loaded);
    for effect in effects {
        let args: Vec<String> = match effect {
            Effect::Engine(command) => command,
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
        let newly_loaded = !state.loaded;
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
        if newly_loaded || snapshot.paused != previous.paused || snapshot.speed != previous.speed {
            ui.set_status(
                format!(
                    "{} · {:.2}×",
                    if snapshot.paused { "Paused" } else { "Playing" },
                    snapshot.speed
                )
                .into(),
            );
        }
    }
    if !snapshot.error.is_empty() {
        ui.set_status(snapshot.error.clone().into());
        ui.set_loaded(false);
        state.borrow_mut().loaded = false;
    }
    if snapshot.tracks != previous.tracks {
        ui.set_has_video(snapshot.tracks.iter().any(|track| track.kind == "video"));
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

fn default_preferences_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.filter(|path| path.is_absolute())
        .map(|base| base.join("Foxide/preferences.json"))
}

fn apply_preferences(
    ui: &PlayerWindow,
    preferences: &Preferences,
    normal_size: &std::cell::Cell<slint::PhysicalSize>,
) {
    ui.set_light_theme(preferences.light_theme());
    let rgb = preferences.accent_rgb();
    let (r, g, b) = ((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
    ui.set_accent_color(slint::Color::from_rgb_u8(r, g, b));
    ui.set_accent_text(
        if u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 >= 128_000 {
            slint::Color::from_rgb_u8(0x17, 0x1a, 0x20)
        } else {
            slint::Color::from_rgb_u8(255, 255, 255)
        },
    );
    ui.set_accent_hex(format!("#{rgb:06x}").into());
    let rows: Vec<_> = preferences
        .bindings()
        .map(|(key, action)| BindingRow {
            key: key.into(),
            action: action.into(),
        })
        .collect();
    ui.set_key_bindings(ModelRc::new(VecModel::from(rows)));
    ui.set_shortcut_actions(ModelRc::new(VecModel::from(
        player_ui::SHORTCUT_ACTIONS
            .iter()
            .map(|entry| slint::SharedString::from(entry.0))
            .collect::<Vec<_>>(),
    )));
    ui.set_shortcut_help(
        preferences
            .bindings()
            .map(|(key, action)| format!("{key}  —  {action}"))
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
    );
    if ui.get_compact_mode() != preferences.compact() {
        if preferences.compact() {
            let current = ui.window().size();
            if current.width > 0 && current.height > 0 {
                normal_size.set(current);
            }
        }
        ui.set_compact_mode(preferences.compact());
        if preferences.compact() {
            ui.window().set_size(slint::LogicalSize::new(560.0, 280.0));
        } else {
            ui.window().set_size(normal_size.get());
        }
    }
}

fn edit_preferences(
    ui: &PlayerWindow,
    preferences: &Rc<RefCell<Preferences>>,
    path: Option<&std::path::Path>,
    normal_size: &std::cell::Cell<slint::PhysicalSize>,
    edit: impl FnOnce(&mut Preferences) -> Result<(), String>,
) {
    let mut candidate = preferences.borrow().clone();
    let result =
        edit(&mut candidate).and_then(|()| path.map_or(Ok(()), |path| candidate.save(path)));
    match result {
        Ok(()) => {
            apply_preferences(ui, &candidate, normal_size);
            *preferences.borrow_mut() = candidate;
            ui.set_preferences_error(
                if path.is_some() {
                    ""
                } else {
                    "Changes apply to this session; no configuration directory is available."
                }
                .into(),
            );
        }
        Err(error) => ui.set_preferences_error(error.into()),
    }
}

fn desktop_key_name(text: &str) -> &str {
    use slint::platform::Key;
    for (key, name) in [
        (Key::LeftArrow, "ArrowLeft"),
        (Key::RightArrow, "ArrowRight"),
        (Key::UpArrow, "ArrowUp"),
        (Key::DownArrow, "ArrowDown"),
        (Key::Return, "Enter"),
        (Key::Tab, "Tab"),
        (Key::F1, "F1"),
        (Key::F2, "F2"),
        (Key::F3, "F3"),
        (Key::F4, "F4"),
        (Key::F5, "F5"),
        (Key::F6, "F6"),
        (Key::F7, "F7"),
        (Key::F8, "F8"),
        (Key::F9, "F9"),
        (Key::F10, "F10"),
        (Key::F11, "F11"),
        (Key::F12, "F12"),
        (Key::F13, "F13"),
        (Key::F14, "F14"),
        (Key::F15, "F15"),
        (Key::F16, "F16"),
        (Key::F17, "F17"),
        (Key::F18, "F18"),
        (Key::F19, "F19"),
        (Key::F20, "F20"),
        (Key::F21, "F21"),
        (Key::F22, "F22"),
        (Key::F23, "F23"),
        (Key::F24, "F24"),
    ] {
        let encoded: slint::SharedString = key.into();
        if encoded.as_str() == text {
            return name;
        }
    }
    text
}

fn install_preferences(
    ui: &PlayerWindow,
    preferences: Rc<RefCell<Preferences>>,
    path: Option<PathBuf>,
) {
    let normal_size = Rc::new(std::cell::Cell::new(slint::PhysicalSize::new(1180, 770)));
    apply_preferences(ui, &preferences.borrow(), &normal_size);
    let (weak, prefs) = (ui.as_weak(), preferences.clone());
    ui.on_shortcut(move |key, ctrl, alt, shift, meta| {
        let command =
            prefs
                .borrow()
                .resolve_key(desktop_key_name(key.as_str()), ctrl, alt, shift, meta);
        if let (Some(ui), Some((name, value))) = (weak.upgrade(), command) {
            ui.invoke_action(name.into(), value as f32);
            true
        } else {
            false
        }
    });
    let (weak, prefs, location, size) = (
        ui.as_weak(),
        preferences.clone(),
        path.clone(),
        normal_size.clone(),
    );
    ui.on_save_binding(move |key, action| {
        if let Some(ui) = weak.upgrade() {
            edit_preferences(&ui, &prefs, location.as_deref(), &size, |prefs| {
                prefs.bind(key.as_str(), action.as_str())
            });
        }
    });
    let (weak, prefs, location, size) = (
        ui.as_weak(),
        preferences.clone(),
        path.clone(),
        normal_size.clone(),
    );
    ui.on_remove_binding(move |key| {
        if let Some(ui) = weak.upgrade() {
            edit_preferences(&ui, &prefs, location.as_deref(), &size, |prefs| {
                prefs.unbind(key.as_str())
            });
        }
    });
    let (weak, prefs, location, size) = (
        ui.as_weak(),
        preferences.clone(),
        path.clone(),
        normal_size.clone(),
    );
    ui.on_save_appearance(move |light, color| {
        if let Some(ui) = weak.upgrade() {
            edit_preferences(&ui, &prefs, location.as_deref(), &size, |prefs| {
                prefs.set_appearance(light, color.as_str())
            });
        }
    });
    let (weak, prefs, location, size) = (
        ui.as_weak(),
        preferences.clone(),
        path.clone(),
        normal_size.clone(),
    );
    ui.on_reset_preferences(move || {
        if let Some(ui) = weak.upgrade() {
            edit_preferences(&ui, &prefs, location.as_deref(), &size, |prefs| {
                *prefs = Preferences::default();
                Ok(())
            });
        }
    });
    let weak = ui.as_weak();
    ui.on_toggle_compact(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_panel("".into());
            edit_preferences(&ui, &preferences, path.as_deref(), &normal_size, |prefs| {
                prefs.set_compact(!prefs.compact());
                Ok(())
            });
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let startup = startup_commands(&args)?;
    let config_path = args.config.clone().or_else(default_preferences_path);
    let (preferences, config_error) = if args.preview_png.is_some() && args.config.is_none() {
        (Preferences::default(), None)
    } else {
        match config_path.as_deref().map(Preferences::load).transpose() {
            Ok(value) => (value.unwrap_or_default(), None),
            Err(error) => (Preferences::default(), Some(error)),
        }
    };
    if let Some(path) = args.preview_png {
        return preview::render(&path, &preferences);
    }
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let ui = PlayerWindow::new()?;
    install_preferences(&ui, Rc::new(RefCell::new(preferences)), config_path);
    if let Some(error) = config_error {
        ui.set_preferences_error(format!("Could not load preferences: {error}").into());
    }
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
    let filter_state = state.clone();
    let filter_window = ui.as_weak();
    ui.on_filter_queue(move |query| {
        if let Some(ui) = filter_window.upgrade() {
            ui.set_queue_filter(query);
            update_queue(&ui, &filter_state.borrow());
        }
    });
    let (weak, controller, playback) = (ui.as_weak(), state.clone(), engine.clone());
    ui.on_action(move |name, value| {
        let Some(ui) = weak.upgrade() else { return };
        let action = match name.as_str() {
            "url" | "preferences" => {
                ui.set_panel(name);
                return;
            }
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
            _ => match Action::control(name.as_str(), value as f64) {
                Some(action) => action,
                None => return,
            },
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
    if let Some(engine) = &engine {
        for command in startup {
            engine.command(command)?;
        }
    }
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

    #[test]
    fn benchmark_startup_is_explicit_and_preserves_speed_and_pause() {
        let args = Args::try_parse_from([
            "foxide-desktop",
            "--benchmark-script",
            "bench.lua",
            "--speed",
            "2",
            "--paused",
            "movie.mp4",
        ])
        .unwrap();
        assert_eq!(
            startup_commands(&args).unwrap(),
            vec![
                vec!["load-script".to_owned(), "bench.lua".to_owned()],
                vec!["set".to_owned(), "speed".to_owned(), "2".to_owned()],
                vec!["set".to_owned(), "pause".to_owned(), "yes".to_owned()],
            ]
        );
        let defaults = Args::try_parse_from(["foxide-desktop"]).unwrap();
        assert!(startup_commands(&defaults).unwrap().is_empty());
        let invalid = Args::try_parse_from(["foxide-desktop", "--speed", "NaN"]).unwrap();
        assert!(startup_commands(&invalid).is_err());
    }
    use slint::{
        Model,
        platform::{
            Platform, WindowAdapter,
            software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        },
    };

    struct TestPlatform(Rc<MinimalSoftwareWindow>);
    impl Platform for TestPlatform {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    fn capture_customization(ui: &PlayerWindow, window: &MinimalSoftwareWindow, name: &str) {
        let Some(folder) = std::env::var_os("FOXIDE_UI_PREVIEW_DIR").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&folder).unwrap();
        let size = ui.window().size();
        let width = size.width as usize;
        let mut pixels = vec![slint::Rgb8Pixel::default(); width * size.height as usize];
        ui.window().request_redraw();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width);
        });
        let bytes: Vec<_> = pixels
            .iter()
            .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
            .collect();
        image::save_buffer(
            folder.join(format!("{name}.png")),
            &bytes,
            size.width,
            size.height,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }

    #[test]
    fn playback_refresh_preserves_visible_state_across_ticks_tracks_and_errors() {
        let rendered_window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(TestPlatform(rendered_window.clone()))).unwrap();
        let ui = PlayerWindow::new().unwrap();
        ui.window().set_size(slint::PhysicalSize::new(1180, 770));
        ui.show().unwrap();
        let preferences = Rc::new(RefCell::new(player_ui::Preferences::default()));
        let settings_path = std::env::temp_dir().join(format!(
            "foxide-desktop-prefs-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct RemoveSettings(PathBuf);
        impl Drop for RemoveSettings {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleanup = RemoveSettings(settings_path.clone());
        install_preferences(&ui, preferences.clone(), Some(settings_path.clone()));
        capture_customization(&ui, &rendered_window, "dark");
        ui.invoke_save_binding("Ctrl+K".into(), "pause".into());
        let actions = Rc::new(RefCell::new(Vec::new()));
        let recorded = actions.clone();
        ui.on_action(move |name, value| recorded.borrow_mut().push((name.to_string(), value)));
        assert!(ui.invoke_shortcut("k".into(), true, false, false, false));
        assert_eq!(&*actions.borrow(), &[("pause".into(), 0.0)]);
        ui.invoke_save_binding("F2".into(), "next".into());
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed {
                text: slint::platform::Key::F2.into(),
            });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased {
                text: slint::platform::Key::F2.into(),
            });
        assert_eq!(
            actions.borrow().last(),
            Some(&("next".into(), 0.0)),
            "Native function keys must reach editable bindings"
        );
        ui.invoke_save_appearance(true, "#3178ce".into());
        assert!(ui.get_light_theme());
        assert_eq!(
            ui.get_accent_color(),
            slint::Color::from_rgb_u8(0x31, 0x78, 0xce)
        );
        capture_customization(&ui, &rendered_window, "light");
        ui.set_panel("preferences".into());
        let before_dialog_key = actions.borrow().len();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: "p".into() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text: "p".into() });
        assert_eq!(
            actions.borrow().len(),
            before_dialog_key,
            "Editing preferences cannot trigger playback shortcuts"
        );
        capture_customization(&ui, &rendered_window, "preferences");
        ui.set_panel("".into());
        let normal_size = ui.window().size();
        ui.invoke_toggle_compact();
        assert!(ui.get_compact_mode());
        assert_eq!(ui.window().size(), slint::PhysicalSize::new(560, 280));
        capture_customization(&ui, &rendered_window, "compact");
        let saved = player_ui::Preferences::load(&settings_path).unwrap();
        assert!(saved.compact());
        assert!(saved.light_theme());
        assert_eq!(
            saved.resolve_key("k", true, false, false, false),
            Some(("pause", 0.0))
        );
        ui.invoke_save_binding("Escape".into(), "quit".into());
        assert!(!ui.get_preferences_error().is_empty());
        assert_eq!(player_ui::Preferences::load(&settings_path).unwrap(), saved);
        ui.invoke_toggle_compact();
        assert!(!ui.get_compact_mode());
        assert_eq!(ui.window().size(), normal_size);
        ui.invoke_remove_binding("Ctrl+K".into());
        assert!(!ui.invoke_shortcut("k".into(), true, false, false, false));
        ui.invoke_reset_preferences();
        assert_eq!(
            player_ui::Preferences::load(&settings_path).unwrap(),
            player_ui::Preferences::default()
        );
        assert!(!ui.get_light_theme());
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
            }]
            .into(),
            ..Default::default()
        };
        refresh(&ui, &state, &snapshot, &mut previous);
        assert!(ui.get_loaded());
        assert_eq!(ui.get_media_title(), "Movie");
        assert_eq!(ui.get_time_label(), "01:05");
        assert_eq!(ui.get_duration_label(), "02:05");
        assert_eq!(ui.get_status(), "Playing · 1.00×");
        assert_eq!(ui.get_audio_tracks().row_count(), 1);
        assert!(
            !ui.get_has_video(),
            "Audio-only playback must not enable frame/video controls"
        );
        update_queue(&ui, &state.borrow());
        assert_eq!(ui.get_queue().row_count(), 1);
        ui.set_queue_filter("missing".into());
        update_queue(&ui, &state.borrow());
        assert_eq!(ui.get_queue().row_count(), 0);
        assert_eq!(ui.get_queue_count(), 1);
        ui.set_queue_filter("movie".into());
        update_queue(&ui, &state.borrow());
        assert_eq!(ui.get_queue().row_data(0).unwrap().source_index, 0);
        let diagnostics = ui.get_diagnostics();
        assert!(diagnostics.contains("Active decoder: not active"));
        ui.set_status("Command rejected".into());
        snapshot.position = 65.9;
        refresh(&ui, &state, &snapshot, &mut previous);
        assert_eq!(ui.get_position(), 65.9);
        assert_eq!(
            ui.get_status(),
            "Command rejected",
            "Position ticks must not rebuild and overwrite unchanged playback status"
        );
        assert_eq!(ui.get_time_label(), "01:05");
        assert_eq!(ui.get_diagnostics(), diagnostics);
        snapshot.position = 66.0;
        snapshot.paused = true;
        snapshot.speed = 1.5;
        snapshot.title = "New title".into();
        snapshot.duration = 126.0;
        snapshot.hwdec = "d3d11va".into();
        Arc::make_mut(&mut snapshot.tracks)[0].selected = false;
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
        state
            .borrow_mut()
            .dispatch(Action::Open(vec!["next.mkv".into()]));
        ui.set_status("Opening media…".into());
        snapshot.path = "next.mkv".into();
        snapshot.error.clear();
        refresh(&ui, &state, &snapshot, &mut previous);
        assert_eq!(
            ui.get_status(),
            "Paused · 1.50×",
            "A newly loaded file replaces the opening status even when pause and speed are unchanged"
        );
    }
}
