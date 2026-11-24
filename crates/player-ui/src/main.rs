mod controller_navigation;
mod picture_in_picture;
mod profile_ui;
mod shader_ui;
mod system_media;
use clap::Parser;
use player_cast::{CastController, CastEvent, CastOptions, CertificatePin};
use player_render::{Engine, Snapshot};
use player_settings::Preferences;
use player_ui::{Action, Effect, UiState};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Duration,
};
// Only pinned Slint compiler output may use its generated vtable unsafe code.
// Hand-written UI glue remains under deny(unsafe_code); FFI lives in player-render.
#[allow(unsafe_code)]
mod generated {
    slint::include_modules!();
}
use generated::*;
mod diagnostics;
mod network_ui;
mod preview;
mod updates_ui;

#[derive(Parser)]
#[command(about = "Foxide desktop player — Slint and embedded libmpv")]
struct Args {
    /// Local media files or a stream URL.
    media: Vec<PathBuf>,
    /// Override the Radio Browser-compatible directory origin.
    #[arg(long)]
    radio_directory: Option<String>,
    /// Explicit trusted yt-dlp executable (overrides the saved helper for this launch).
    #[arg(long)]
    yt_dlp: Option<PathBuf>,
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

fn install_diagnostic_log_handler(
    ui: &PlayerWindow,
    open: impl Fn() -> Result<(), String> + 'static,
) {
    let weak = ui.as_weak();
    ui.on_open_diagnostic_log(move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Err(error) = open() {
            diagnostics::record("Could not open diagnostic log");
            ui.set_status(format!("Could not open diagnostic log: {error}").into());
            eprintln!("Could not open diagnostic log: {error}");
        }
    });
}

fn install_playback_diagnostics_handler(ui: &PlayerWindow) {
    let weak = ui.as_weak();
    ui.on_open_playback_diagnostics(move || {
        if let Some(ui) = weak.upgrade() {
            ui.set_panel("information".into());
        }
    });
}

fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{:02}:{:02}", total / 60, total % 60)
    }
}

fn intro_skip_target(snapshot: &Snapshot) -> Option<f64> {
    if !snapshot.loaded
        || !snapshot.position.is_finite()
        || !snapshot.duration.is_finite()
        || snapshot.position < 0.0
        || snapshot.duration <= 0.0
    {
        return None;
    }
    let current = snapshot.chapters.get(snapshot.chapter_index)?;
    let next = snapshot.chapters.get(snapshot.chapter_index + 1)?;
    if !is_intro_marker(&current.title)
        || !current.start.is_finite()
        || !next.start.is_finite()
        || snapshot.position < current.start
        || next.start <= snapshot.position
        || next.start <= current.start
        || next.start > snapshot.duration
    {
        return None;
    }
    Some(next.start)
}

fn introdb_identity(snapshot: &Snapshot) -> Option<player_services::EpisodeIdentity> {
    let season = snapshot.season.trim().parse::<u32>().ok()?;
    let episode = snapshot.episode.trim().parse::<u32>().ok()?;
    player_services::EpisodeIdentity::new(snapshot.imdb_id.trim(), season, episode).ok()
}

fn online_intro_skip_target(
    snapshot: &Snapshot,
    imdb_id: &str,
    season: u32,
    episode: u32,
    start: f64,
    end: f64,
) -> Option<f64> {
    let identity = introdb_identity(snapshot)?;
    if identity.imdb_id() != imdb_id
        || identity.season() != season
        || identity.episode() != episode
        || !snapshot.loaded
        || !snapshot.position.is_finite()
        || !snapshot.duration.is_finite()
        || !start.is_finite()
        || !end.is_finite()
        || start < 0.0
        || end <= start
        || end > snapshot.duration
        || snapshot.position < start
        || snapshot.position > end
    {
        return None;
    }
    Some(end)
}

fn is_intro_marker(title: &str) -> bool {
    let mut previous = "";
    for word in title.split(|character: char| !character.is_alphanumeric()) {
        if word.eq_ignore_ascii_case("intro")
            || word.eq_ignore_ascii_case("recap")
            || (previous.eq_ignore_ascii_case("opening") && word.eq_ignore_ascii_case("credits"))
            || (previous.eq_ignore_ascii_case("previously") && word.eq_ignore_ascii_case("on"))
        {
            return true;
        }
        previous = word;
    }
    false
}

fn update_queue(ui: &PlayerWindow, state: &UiState) {
    let rows = state
        .filtered_indices(ui.get_queue_filter().as_str())
        .into_iter()
        .map(|index| {
            let path = &state.playlist[index];
            if path.to_string_lossy() == ui.get_active_radio_url().as_str() {
                return QueueRow {
                    source_index: index as i32,
                    label: ui.get_active_radio_name(),
                };
            }
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
    ui.set_cast_available(
        state.loaded
            && state
                .playlist
                .get(state.selected)
                .is_some_and(|path| path.is_file() && supported_cast_extension(path)),
    );
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
    preview_generation: &Cell<u64>,
    snapshot: &Snapshot,
    previous: &mut Snapshot,
) {
    if snapshot == previous {
        return;
    }
    if snapshot.path != previous.path
        || snapshot.imdb_id != previous.imdb_id
        || snapshot.season != previous.season
        || snapshot.episode != previous.episode
    {
        ui.set_media_generation(ui.get_media_generation().wrapping_add(1));
        ui.set_introdb_imdb_id("".into());
        ui.set_introdb_season(0);
        ui.set_introdb_episode(0);
        ui.set_introdb_busy(false);
        ui.set_online_intro_start(-1.0);
        ui.set_online_intro_end(-1.0);
        ui.set_introdb_status("".into());
    }
    let identity = introdb_identity(snapshot);
    ui.set_introdb_available(snapshot.loaded && identity.is_some());
    ui.set_current_imdb_id(identity.as_ref().map_or("", |value| value.imdb_id()).into());
    ui.set_current_season(identity.as_ref().map_or(0, |value| value.season() as i32));
    ui.set_current_episode(identity.as_ref().map_or(0, |value| value.episode() as i32));
    ui.set_skip_intro_available(
        intro_skip_target(snapshot).is_some()
            || online_intro_skip_target(
                snapshot,
                ui.get_introdb_imdb_id().as_str(),
                ui.get_introdb_season().max(0) as u32,
                ui.get_introdb_episode().max(0) as u32,
                f64::from(ui.get_online_intro_start()),
                f64::from(ui.get_online_intro_end()),
            )
            .is_some(),
    );
    let expected = state
        .borrow()
        .playlist
        .get(state.borrow().selected)
        .map(|p| {
            player_core::external_path(p.clone())
                .to_string_lossy()
                .into_owned()
        });
    let matches_selection = expected
        .as_deref()
        .is_some_and(|path| path == snapshot.path || path == snapshot.requested_path.as_ref());
    if snapshot.loaded && matches_selection {
        let mut state = state.borrow_mut();
        let newly_loaded = !state.loaded;
        if newly_loaded {
            preview_generation.set(preview_generation.get().wrapping_add(1));
        }
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
        if newly_loaded
            || snapshot.paused != previous.paused
            || snapshot.speed != previous.speed
            || (!previous.command_error.is_empty() && snapshot.command_error.is_empty())
        {
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
    if !snapshot.command_error.is_empty() && snapshot.command_error != previous.command_error {
        ui.set_status(snapshot.command_error.clone().into());
    }
    if !snapshot.error.is_empty() {
        ui.set_status(snapshot.error.clone().into());
        ui.set_loaded(false);
        state.borrow_mut().loaded = false;
    }
    let state_snapshot = state.borrow();
    ui.set_cast_available(
        snapshot.loaded
            && matches_selection
            && state_snapshot.loaded
            && state_snapshot
                .playlist
                .get(state_snapshot.selected)
                .is_some_and(|path| path.is_file() && supported_cast_extension(path)),
    );
    drop(state_snapshot);
    ui.set_has_artwork(
        ui.get_loaded()
            && snapshot.loaded
            && snapshot.has_artwork
            && matches_selection
            && snapshot.error.is_empty(),
    );
    if snapshot.tracks != previous.tracks {
        ui.set_has_video(
            snapshot
                .tracks
                .iter()
                .any(|track| track.kind == "video" && track.selected),
        );
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
    ui.set_video_site_helper(preferences.video_site_helper().unwrap_or("").into());
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
        player_settings::SHORTCUT_ACTIONS
            .iter()
            .map(|entry| slint::SharedString::from(entry.0))
            .collect::<Vec<_>>(),
    )));
    let mut shortcut_help = preferences
        .bindings()
        .map(|(key, action)| format!("{key}  —  {action}"))
        .collect::<Vec<_>>()
        .join("\n");
    shortcut_help.push_str(
        "\n\nGame controller\nD-pad: move focus · South (A/Cross): select · East/Select: back\nStart: play/pause · Left/right shoulder: previous/next",
    );
    ui.set_shortcut_help(shortcut_help.into());
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
    ui.on_save_video_site_helper(move |chosen| {
        if let Some(ui) = weak.upgrade() {
            let selection = if chosen.is_empty() {
                Ok(None)
            } else {
                player_services::VideoSiteTool::new(std::path::Path::new(chosen.as_str())).map(Some)
            };
            match selection {
                Ok(tool) => {
                    edit_preferences(&ui, &prefs, location.as_deref(), &size, |prefs| {
                        prefs.set_video_site_helper(
                            tool.as_ref().and_then(|tool| tool.path().to_str()),
                        )
                    });
                    if ui.get_preferences_error().is_empty() {
                        ui.set_video_site_notice("Restart Foxide to apply this change".into());
                    }
                }
                Err(error) => ui.set_preferences_error(error.into()),
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_choose_video_site_helper(move || {
        let weak = weak.clone();
        std::thread::spawn(move || {
            let dialog = rfd::AsyncFileDialog::new().set_title("Choose trusted yt-dlp executable");
            #[cfg(windows)]
            let dialog = dialog.add_filter("Native executable", &["exe"]);
            if let Some(file) = futures_lite::future::block_on(dialog.pick_file()) {
                let path = file.path().to_path_buf();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    if let Some(path) = path.to_str() {
                        ui.invoke_save_video_site_helper(path.into());
                    } else {
                        ui.set_preferences_error("Helper path must be Unicode".into());
                    }
                });
            }
        });
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

#[cfg(test)]
mod cast_endpoint_tests {
    use super::{cast_seek_target, parse_cast_address, supported_cast_extension};
    use player_cast::{CastState, CastStatus};
    use std::path::Path;

    #[test]
    fn cast_skip_targets_stay_inside_the_known_media_duration() {
        let status = CastStatus {
            state: CastState::Playing,
            position: 4.0,
            duration: Some(12.0),
        };
        assert_eq!(cast_seek_target(&status, -10.0), Some(0.0));
        assert_eq!(cast_seek_target(&status, 10.0), Some(12.0));
        let unknown_duration = CastStatus {
            duration: None,
            ..status
        };
        assert_eq!(cast_seek_target(&unknown_duration, 10.0), Some(14.0));
    }

    #[test]
    fn accepts_literal_local_receiver_addresses() {
        assert_eq!(
            parse_cast_address("192.168.1.20:8009").unwrap().to_string(),
            "192.168.1.20:8009"
        );
    }

    #[test]
    fn rejects_nonlocal_or_nonliteral_receiver_addresses() {
        for input in ["8.8.8.8:8009", "receiver.local:8009", "127.0.0.1:0"] {
            assert!(parse_cast_address(input).is_err(), "accepted {input}");
        }
    }

    #[test]
    fn accepts_local_ipv6_receiver_addresses_and_rejects_global_ipv6() {
        assert!(parse_cast_address("[fd00::20]:8009").is_ok());
        assert!(parse_cast_address("[fe80::20]:8009").is_err());
        assert!(parse_cast_address("[2001:4860:4860::8888]:8009").is_err());
    }

    #[test]
    fn supports_only_the_containers_served_by_the_cast_origin() {
        for path in [
            "movie.mp4",
            "clip.M4V",
            "video.webm",
            "movie.mkv",
            "song.mp3",
            "song.ogg",
            "song.opus",
            "song.flac",
            "song.wav",
        ] {
            assert!(supported_cast_extension(Path::new(path)), "rejected {path}");
        }
        for path in ["stream.m3u8", "movie.avi", "movie.mp4.txt", "unknown"] {
            assert!(
                !supported_cast_extension(Path::new(path)),
                "accepted {path}"
            );
        }
    }
}

fn parse_cast_address(input: &str) -> Result<std::net::SocketAddr, ()> {
    let address = input.parse::<std::net::SocketAddr>().map_err(|_| ())?;
    let local = match address.ip() {
        std::net::IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        std::net::IpAddr::V6(ip) => {
            ip.is_unique_local()
                || ip.is_loopback()
                || (ip.is_unicast_link_local()
                    && matches!(address, std::net::SocketAddr::V6(v6) if v6.scope_id() != 0))
        }
    };
    if !local || address.port() == 0 {
        return Err(());
    }
    Ok(address)
}

fn supported_cast_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp4" | "m4v" | "webm" | "mkv" | "mp3" | "ogg" | "opus" | "flac" | "wav"
            )
        })
}

fn cast_seek_target(status: &player_cast::CastStatus, offset: f64) -> Option<f64> {
    if !status.position.is_finite() || !offset.is_finite() {
        return None;
    }
    let duration = status
        .duration
        .filter(|duration| duration.is_finite() && *duration >= 0.0)
        .unwrap_or(f64::MAX);
    Some((status.position + offset).clamp(0.0, duration))
}

fn cast_receiver_rows(preferences: &Preferences) -> Vec<CastReceiverRow> {
    preferences
        .cast_receivers()
        .iter()
        .map(|receiver| CastReceiverRow {
            name: receiver.name().into(),
            endpoint: receiver.endpoint().into(),
            fingerprint: receiver.fingerprint().into(),
        })
        .collect()
}

fn save_cast_preference(
    ui: &PlayerWindow,
    preferences: &Rc<RefCell<Preferences>>,
    path: Option<&std::path::Path>,
    edit: impl FnOnce(&mut Preferences) -> Result<(), String>,
) {
    let Some(path) = path else {
        ui.set_cast_status("Receiver preferences cannot be saved because no local configuration path is available.".into());
        return;
    };
    let mut candidate = preferences.borrow().clone();
    match edit(&mut candidate).and_then(|()| candidate.save(path)) {
        Ok(()) => {
            ui.set_cast_paired_receivers(ModelRc::new(VecModel::from(cast_receiver_rows(
                &candidate,
            ))));
            *preferences.borrow_mut() = candidate;
            ui.set_cast_status("Remembered receivers were saved in local preferences. Certificate changes still require explicit review.".into());
        }
        Err(_) => ui.set_cast_status(
            "Could not save receiver preferences. No receiver settings were changed.".into(),
        ),
    }
}

#[cfg(test)]
fn install_cast(ui: &PlayerWindow, state: &Rc<RefCell<UiState>>) {
    install_cast_with_preferences(
        ui,
        state,
        Rc::new(RefCell::new(Preferences::default())),
        None,
    );
}

fn install_cast_with_preferences(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    preferences: Rc<RefCell<Preferences>>,
    preferences_path: Option<PathBuf>,
) {
    install_cast_with_discovery(
        ui,
        state,
        preferences,
        preferences_path,
        player_cast::discover_receivers,
    );
}

fn install_cast_with_discovery(
    ui: &PlayerWindow,
    state: &Rc<RefCell<UiState>>,
    preferences: Rc<RefCell<Preferences>>,
    preferences_path: Option<PathBuf>,
    discover: impl Fn(std::time::Duration) -> std::io::Result<Vec<player_cast::CastReceiver>>
    + Send
    + Sync
    + 'static,
) {
    ui.set_cast_paired_receivers(ModelRc::new(VecModel::from(cast_receiver_rows(
        &preferences.borrow(),
    ))));
    let weak = ui.as_weak();
    ui.on_cast_select(move |name, endpoint, fingerprint| {
        if let Some(ui) = weak.upgrade() {
            ui.set_cast_name(name);
            ui.set_cast_endpoint(endpoint);
            ui.set_cast_fingerprint(fingerprint.clone());
            ui.set_cast_status(
                if fingerprint.is_empty() {
                    "Receiver selected. Inspect its certificate and review the fingerprint before casting.".into()
                } else {
                    "Remembered receiver selected. Its saved fingerprint is required for casting; inspect again if you need to re-pair.".into()
                },
            );
        }
    });
    let active: Arc<std::sync::Mutex<Option<CastController>>> =
        Arc::new(std::sync::Mutex::new(None));
    let last_status: Arc<std::sync::Mutex<Option<player_cast::CastStatus>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (weak, state, active_for_connect, status_for_connect) = (
        ui.as_weak(),
        state.clone(),
        active.clone(),
        last_status.clone(),
    );
    ui.on_cast_connect(move |endpoint, fingerprint| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if active_for_connect
            .lock()
            .expect("Cast controller lock poisoned")
            .is_some()
        {
            ui.set_cast_status("A cast session is already active".into());
            return;
        }
        let address = match parse_cast_address(&endpoint) {
            Ok(address) => address,
            Err(_) => {
                ui.set_cast_status(
                    "Enter a literal local receiver address and port, such as 192.168.1.20:8009"
                        .into(),
                );
                return;
            }
        };
        let pin = match fingerprint.parse::<CertificatePin>() {
            Ok(pin) => pin,
            Err(_) => {
                ui.set_cast_status(
                    "Enter a previously inspected 64 digit SHA-256 certificate fingerprint".into(),
                );
                return;
            }
        };
        let selected = {
            let state = state.borrow();
            state.playlist.get(state.selected).cloned()
        };
        let Some(path) = selected.filter(|path| path.is_file() && supported_cast_extension(path))
        else {
            ui.set_cast_status("Choose a supported local media file before casting".into());
            return;
        };
        ui.set_cast_status("Connecting to the explicitly pinned receiver…".into());
        let event_window = weak.clone();
        let active_for_events = active_for_connect.clone();
        let status_for_events = status_for_connect.clone();
        let controller =
            CastController::start_file(path, address, pin, CastOptions::default(), move |event| {
                let active_for_event = active_for_events.clone();
                let status_for_event = status_for_events.clone();
                match &event {
                    CastEvent::Status(status) => {
                        *status_for_event.lock().expect("Cast status lock poisoned") =
                            Some(status.clone());
                    }
                    CastEvent::Stopped | CastEvent::Failed(_) => {
                        *status_for_event.lock().expect("Cast status lock poisoned") = None;
                    }
                    CastEvent::Rejected(_) => {}
                }
                let _ = event_window.upgrade_in_event_loop(move |ui| match event {
                    CastEvent::Status(status) => {
                        ui.set_cast_active(true);
                        ui.set_cast_paused(status.state == player_cast::CastState::Paused);
                        ui.set_cast_status(
                            format!("{:?} · {:.1} seconds", status.state, status.position).into(),
                        );
                    }
                    CastEvent::Stopped => {
                        ui.set_cast_active(false);
                        ui.set_cast_paused(false);
                        *active_for_event
                            .lock()
                            .expect("Cast controller lock poisoned") = None;
                        ui.set_cast_status("Casting stopped".into());
                    }
                    CastEvent::Rejected(message) => {
                        ui.set_cast_status(format!("Cast command rejected: {message}").into())
                    }
                    CastEvent::Failed(message) => {
                        ui.set_cast_active(false);
                        ui.set_cast_paused(false);
                        *active_for_event
                            .lock()
                            .expect("Cast controller lock poisoned") = None;
                        ui.set_cast_status(format!("Cast failed: {message}").into());
                    }
                });
            });
        match controller {
            Ok(controller) => {
                *active_for_connect
                    .lock()
                    .expect("Cast controller lock poisoned") = Some(controller)
            }
            Err(error) => {
                ui.set_cast_status(format!("Could not start Cast worker: {error}").into())
            }
        }
    });
    let weak = ui.as_weak();
    ui.on_cast_inspect(move |endpoint| {
        let Some(ui) = weak.upgrade() else { return };
        let address = match parse_cast_address(&endpoint) {
            Ok(address) => address,
            Err(()) => {
                ui.set_cast_status("Enter a literal local receiver address and port before inspecting.".into());
                return;
            }
        };
        ui.set_cast_inspecting(true);
        ui.set_cast_status("Inspecting the receiver certificate; this sends no Cast command…".into());
        let window = weak.clone();
        std::thread::spawn(move || {
            let result = player_cast::inspect_receiver_certificate(
                address,
                std::time::Duration::from_secs(8),
            );
            let _ = window.upgrade_in_event_loop(move |ui| {
                ui.set_cast_inspecting(false);
                match result {
                    Ok(pin) => {
                        ui.set_cast_fingerprint(pin.to_string().into());
                        ui.set_cast_status("Certificate observed. Review the fingerprint, then explicitly start casting to trust this receiver for this session.".into());
                    }
                    Err(_) => ui.set_cast_status("Certificate inspection failed or timed out; no receiver was trusted.".into()),
                }
            });
        });
    });
    let (weak, discover) = (ui.as_weak(), Arc::new(discover));
    ui.on_cast_discover(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_cast_discovering() {
            return;
        }
        ui.set_cast_discovering(true);
        ui.set_cast_receivers(ModelRc::new(VecModel::from(Vec::<CastReceiverRow>::new())));
        ui.set_cast_status("Searching the local network for Cast receivers for up to five seconds…".into());
        let (window, discover) = (weak.clone(), discover.clone());
        std::thread::spawn(move || {
            let result = discover(std::time::Duration::from_secs(5));
            let _ = window.upgrade_in_event_loop(move |ui| {
                ui.set_cast_discovering(false);
                match result {
                    Ok(receivers) => {
                        let rows = receivers
                            .iter()
                            .map(|receiver| CastReceiverRow {
                                name: receiver.name.clone().into(),
                                endpoint: receiver.address.to_string().into(),
                                fingerprint: "".into(),
                            })
                            .collect::<Vec<_>>();
                        ui.set_cast_receivers(ModelRc::new(VecModel::from(rows)));
                        let count = receivers.len();
                        ui.set_cast_status(
                            if count == 0 {
                                "No local Cast receivers found. You can enter an address manually.".into()
                            } else {
                                format!("Found {count} local receiver(s). Select one, inspect its certificate, then review the fingerprint before casting.").into()
                            },
                        );
                    }
                    Err(_) => ui.set_cast_status("Receiver discovery is unavailable on this network. You can enter a local address manually.".into()),
                }
            });
        });
    });
    let (weak, active_for_control, status_for_control) = (ui.as_weak(), active, last_status);
    ui.on_cast_control(move |command| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        match command.as_str() {
            "stop" => {
                if let Some(controller) = active_for_control
                    .lock()
                    .expect("Cast controller lock poisoned")
                    .as_ref()
                {
                    controller.stop();
                    ui.set_cast_status("Stopping cast session…".into());
                }
            }
            "pause" => {
                if let Some(controller) = active_for_control
                    .lock()
                    .expect("Cast controller lock poisoned")
                    .as_ref()
                    && let Err(error) = controller.pause()
                {
                    ui.set_cast_status(error.to_string().into());
                }
            }
            "play" => {
                if let Some(controller) = active_for_control
                    .lock()
                    .expect("Cast controller lock poisoned")
                    .as_ref()
                    && let Err(error) = controller.play()
                {
                    ui.set_cast_status(error.to_string().into());
                }
            }
            "seek-back" | "seek-forward" => {
                let status = status_for_control
                    .lock()
                    .expect("Cast status lock poisoned")
                    .clone();
                let offset = if command == "seek-back" { -10.0 } else { 10.0 };
                let Some(target) = status
                    .as_ref()
                    .and_then(|status| cast_seek_target(status, offset))
                else {
                    ui.set_cast_status("Casting position is not available yet".into());
                    return;
                };
                if let Some(controller) = active_for_control
                    .lock()
                    .expect("Cast controller lock poisoned")
                    .as_ref()
                    && let Err(error) = controller.seek(target)
                {
                    ui.set_cast_status(error.to_string().into());
                }
            }
            _ => {}
        }
    });

    let (weak, remember_preferences, path) =
        (ui.as_weak(), preferences.clone(), preferences_path.clone());
    ui.on_cast_remember(move |name, endpoint, fingerprint| {
        if let Some(ui) = weak.upgrade() {
            let name = if name.trim().is_empty() {
                endpoint.to_string()
            } else {
                name.to_string()
            };
            save_cast_preference(&ui, &remember_preferences, path.as_deref(), |preferences| {
                preferences.remember_cast_receiver(&name, endpoint.as_str(), fingerprint.as_str())
            });
        }
    });

    let (weak, preferences, path) = (ui.as_weak(), preferences, preferences_path);
    ui.on_cast_forget(move |endpoint| {
        if let Some(ui) = weak.upgrade() {
            save_cast_preference(&ui, &preferences, path.as_deref(), |preferences| {
                if preferences.forget_cast_receiver(endpoint.as_str()) {
                    Ok(())
                } else {
                    Err("No remembered receiver matched the selected address".into())
                }
            });
        }
    });
}

fn install_controls(ui: &PlayerWindow, state: &Rc<RefCell<UiState>>, engine: &Option<Rc<Engine>>) {
    let pip_mode = Rc::new(RefCell::new(picture_in_picture::PictureInPictureMode::Main));
    let filter_state = state.clone();
    let filter_window = ui.as_weak();
    ui.on_filter_queue(move |query| {
        if let Some(ui) = filter_window.upgrade() {
            ui.set_queue_filter(query);
            update_queue(&ui, &filter_state.borrow());
        }
    });
    let (weak, controller, playback, pip_mode) =
        (ui.as_weak(), state.clone(), engine.clone(), pip_mode);
    ui.on_action(move |name, value| {
        let Some(ui) = weak.upgrade() else { return };
        if name == "picture-in-picture" {
            toggle_picture_in_picture(&ui, &pip_mode);
            return;
        }
        if matches!(name.as_str(), "frame-step" | "frame-back-step")
            && (!ui.get_loaded() || !ui.get_has_video())
        {
            return;
        }
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
            "skip-intro" => {
                let snapshot = playback.as_ref().and_then(|engine| engine.snapshot());
                let target = snapshot.as_ref().and_then(|snapshot| {
                    intro_skip_target(snapshot).or_else(|| {
                        online_intro_skip_target(
                            snapshot,
                            ui.get_introdb_imdb_id().as_str(),
                            ui.get_introdb_season().max(0) as u32,
                            ui.get_introdb_episode().max(0) as u32,
                            f64::from(ui.get_online_intro_start()),
                            f64::from(ui.get_online_intro_end()),
                        )
                    })
                });
                let Some(target) = target else {
                    ui.set_skip_intro_available(false);
                    return;
                };
                Action::SeekAbsolute(target)
            }
            "lookup-intro" => {
                if ui.get_introdb_busy() {
                    return;
                }
                let Some(snapshot) = playback.as_ref().and_then(|engine| engine.snapshot()) else {
                    ui.set_introdb_status("No episode is loaded".into());
                    return;
                };
                let Some(identity) = introdb_identity(&snapshot) else {
                    ui.set_introdb_status(
                        "Add IMDb ID, season and episode tags to look up markers".into(),
                    );
                    return;
                };
                let (imdb_id, season, episode, generation) = (
                    identity.imdb_id().to_owned(),
                    identity.season(),
                    identity.episode(),
                    ui.get_media_generation(),
                );
                let weak = weak.clone();
                ui.set_introdb_busy(true);
                ui.set_introdb_status("Looking up episode markers…".into());
                std::thread::spawn(move || {
                    let result = player_services::CurlClient::system().and_then(|client| {
                        player_services::IntroDb::public().lookup_intro(
                            &client,
                            &player_services::EpisodeIdentity::new(&imdb_id, season, episode)?,
                            &std::sync::atomic::AtomicBool::new(false),
                        )
                    });
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        if ui.get_media_generation() != generation
                            || ui.get_current_imdb_id().as_str() != imdb_id
                            || ui.get_current_season().max(0) as u32 != season
                            || ui.get_current_episode().max(0) as u32 != episode
                        {
                            return;
                        }
                        ui.set_introdb_busy(false);
                        match result {
                            Ok(Some(segment)) => {
                                ui.set_introdb_imdb_id(imdb_id.into());
                                ui.set_introdb_season(season as i32);
                                ui.set_introdb_episode(episode as i32);
                                ui.set_online_intro_start(segment.start_seconds as f32);
                                ui.set_online_intro_end(segment.end_seconds as f32);
                                ui.set_introdb_status("Intro markers found".into());
                                let target = segment.end_seconds.is_finite()
                                    && segment.start_seconds.is_finite()
                                    && segment.start_seconds >= 0.0
                                    && segment.end_seconds > segment.start_seconds
                                    && segment.end_seconds <= f64::from(ui.get_duration())
                                    && f64::from(ui.get_position()) >= segment.start_seconds
                                    && f64::from(ui.get_position()) <= segment.end_seconds;
                                if target {
                                    ui.set_skip_intro_available(true);
                                }
                            }
                            Ok(None) => ui.set_introdb_status("No intro marker found".into()),
                            Err(_) => ui.set_introdb_status("IntroDB lookup failed".into()),
                        }
                    });
                });
                return;
            }
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
}

fn update_thumbnail_preview(
    ui: &PlayerWindow,
    preview: &player_render::ThumbnailController,
    snapshot: Option<&Snapshot>,
    generation: u64,
    position: f32,
) -> bool {
    let Some(snapshot) = snapshot.filter(|snapshot| {
        snapshot.loaded
            && snapshot.duration.is_finite()
            && snapshot.duration > 0.0
            && snapshot
                .tracks
                .iter()
                .any(|track| track.kind == "video" && track.selected)
    }) else {
        preview.clear();
        ui.set_thumbnail_available(false);
        return false;
    };
    let position = f64::from(position);
    ui.set_thumbnail_label(clock(position.clamp(0.0, snapshot.duration)).into());
    let accepted = preview.request(
        snapshot.path.clone(),
        generation,
        position,
        snapshot.duration,
    );
    if !accepted {
        ui.set_thumbnail_available(false);
    }
    accepted
}

fn toggle_picture_in_picture(
    ui: &PlayerWindow,
    mode: &Rc<RefCell<picture_in_picture::PictureInPictureMode>>,
) {
    use picture_in_picture::{PictureInPictureMode, WindowConfiguration, WindowEffect};
    let window = ui.window();
    let current_mode = *mode.borrow();
    match current_mode {
        PictureInPictureMode::Main => {
            let size = window.size();
            let position = window.position();
            let current = WindowConfiguration {
                width: size.width.max(1),
                height: size.height.max(1),
                x: position.x,
                y: position.y,
                fullscreen: window.is_fullscreen(),
                maximized: window.is_maximized(),
                compact: ui.get_compact_mode(),
            };
            let (next_mode, effect) = PictureInPictureMode::enter(current);
            *mode.borrow_mut() = next_mode;
            ui.set_panel("".into());
            ui.set_queue_visible(false);
            ui.set_fullscreen_mode(false);
            ui.set_compact_mode(false);
            window.set_fullscreen(false);
            window.set_maximized(false);
            ui.set_picture_in_picture(true);
            if let WindowEffect::Resize { width, height } = effect {
                window.set_size(slint::LogicalSize::new(width as f32, height as f32));
            }
        }
        PictureInPictureMode::PictureInPicture(previous) => {
            let Some((next_mode, WindowEffect::Restore(previous))) =
                PictureInPictureMode::PictureInPicture(previous).exit()
            else {
                return;
            };
            *mode.borrow_mut() = next_mode;
            ui.set_picture_in_picture(false);
            window.set_fullscreen(false);
            window.set_maximized(false);
            window.set_size(slint::PhysicalSize::new(previous.width, previous.height));
            window.set_position(slint::PhysicalPosition::new(previous.x, previous.y));
            ui.set_compact_mode(previous.compact);
            ui.set_fullscreen_mode(previous.fullscreen);
            if previous.maximized {
                window.set_maximized(true);
            }
            if previous.fullscreen {
                window.set_fullscreen(true);
            }
        }
    }
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
    diagnostics::record("Application started");
    install_playback_diagnostics_handler(&ui);
    install_diagnostic_log_handler(&ui, || {
        diagnostics::open_log(diagnostics::log_path, |path| {
            diagnostics::open_system_log(path, |command| {
                command
                    .spawn()
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            })
        })
    });
    let selected_helper = args
        .yt_dlp
        .as_deref()
        .or_else(|| preferences.video_site_helper().map(std::path::Path::new));
    let helper = selected_helper
        .map(player_services::VideoSiteTool::new)
        .transpose();
    let helper = match helper {
        Ok(tool) => tool,
        Err(error) => {
            ui.set_preferences_error(format!("Video sites disabled: {error}").into());
            ui.set_video_site_notice("Choose a valid helper under View → Preferences".into());
            None
        }
    };
    let preferences = Rc::new(RefCell::new(preferences));
    install_preferences(&ui, preferences.clone(), config_path.clone());
    if let Some(error) = config_error {
        ui.set_preferences_error(format!("Could not load preferences: {error}").into());
    }
    let radio_api = args
        .radio_directory
        .as_deref()
        .map(player_services::RadioDirectory::new)
        .transpose()?
        .unwrap_or_else(player_services::RadioDirectory::public);
    let _radio = network_ui::install_radio(&ui, Arc::new(network_ui::SystemHttp), radio_api);
    updates_ui::install(&ui);
    let state = Rc::new(RefCell::new(UiState::default()));
    let preview_generation = Rc::new(Cell::new(0));
    install_cast_with_preferences(&ui, &state, preferences, config_path);
    let startup_engine = Engine::with_video_site_helper(helper.as_ref().map(|tool| tool.path()));
    let startup_engine = match startup_engine {
        Err(error) if helper.is_some() => {
            ui.set_preferences_error(format!("Video sites unavailable: {error}").into());
            ui.set_video_site_notice(
                "Local playback remains available; choose a compatible engine and restart".into(),
            );
            Engine::new()
        }
        result => result,
    };
    let engine = match startup_engine {
        Ok(engine) => Some(Rc::new(engine)),
        Err(error) => {
            diagnostics::record(&format!("Playback engine initialization failed: {error}"));
            ui.set_status(error.clone().into());
            ui.set_diagnostics(error.into());
            None
        }
    };
    let thumbnails = Arc::new(player_render::ThumbnailController::new());
    if engine.is_some() {
        diagnostics::record("Playback engine initialized");
    }
    if let Some(engine) = &engine {
        let weak_size = ui.as_weak();
        let weak_frame = ui.as_weak();
        let weak_error = ui.as_weak();
        let weak_wake = ui.as_weak();
        player_render::attach_with_thumbnails(
            ui.window(),
            engine,
            thumbnails.clone(),
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
            {
                let weak = ui.as_weak();
                move |frame, available| {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_thumbnail_frame(frame);
                        ui.set_thumbnail_available(available);
                    }
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
    if let Some(playback) = engine.clone() {
        let weak = ui.as_weak();
        let preview = thumbnails.clone();
        let generation = preview_generation.clone();
        ui.on_thumbnail_hover(move |position| {
            if let Some(ui) = weak.upgrade() {
                let snapshot = playback.snapshot();
                update_thumbnail_preview(
                    &ui,
                    &preview,
                    snapshot.as_ref(),
                    generation.get(),
                    position,
                );
            }
        });
        let weak = ui.as_weak();
        let preview = thumbnails.clone();
        ui.on_thumbnail_leave(move || {
            preview.clear();
            if let Some(ui) = weak.upgrade() {
                ui.set_thumbnail_available(false);
                ui.set_thumbnail_label(String::new().into());
            }
        });
    }
    install_controls(&ui, &state, &engine);
    let _controller_timer = controller_navigation::install(&ui, state.clone(), engine.clone());
    shader_ui::install(&ui, &engine);
    profile_ui::install(&ui, &engine);
    if let Some(engine) = &engine {
        for command in startup {
            engine.command(command)?;
        }
    }
    if !args.media.is_empty() {
        dispatch(&ui, &state, &engine, Action::Open(args.media));
    }
    if let Some(playback) = engine.clone() {
        let weak = ui.as_weak();
        let controller = state.clone();
        let previous = RefCell::new(Snapshot::default());
        let status_source = playback.clone();
        let media_engine = Some(playback.clone());
        let media_session = Rc::new(RefCell::new(None));
        let session_for_refresh = media_session.clone();
        let media_attach_retry = Rc::new(RefCell::new(system_media::AttachRetry::default()));
        let retry_for_refresh = media_attach_retry.clone();
        let generation_for_refresh = preview_generation.clone();
        ui.on_native_snapshot_ready(move || {
            if let Some(ui) = weak.upgrade() {
                if let Some(snapshot) = status_source.snapshot() {
                    if snapshot.path != previous.borrow().path {
                        thumbnails.clear();
                        ui.set_thumbnail_available(false);
                    }
                    refresh(
                        &ui,
                        &controller,
                        &generation_for_refresh,
                        &snapshot,
                        &mut previous.borrow_mut(),
                    );
                    if retry_for_refresh
                        .borrow_mut()
                        .due(std::time::Instant::now())
                    {
                        match system_media::attach(&ui, &controller, &media_engine) {
                            Ok(system_media::AttachOutcome::Attached(session)) => {
                                *session_for_refresh.borrow_mut() = Some(session);
                                retry_for_refresh.borrow_mut().finish();
                            }
                            Ok(system_media::AttachOutcome::RetryableNotReady) => {}
                            Err(error) => {
                                retry_for_refresh.borrow_mut().finish();
                                ui.set_status(
                                    format!("System media controls unavailable: {error}").into(),
                                );
                            }
                        }
                    }
                    if let Some(session) = session_for_refresh.borrow_mut().as_mut() {
                        match session.publish(&snapshot, &controller.borrow()) {
                            Ok(()) => session.last_error = None,
                            Err(error) if session.last_error.as_deref() != Some(error.as_str()) => {
                                session.last_error = Some(error.clone());
                                ui.set_status(
                                    format!("System media controls unavailable: {error}").into(),
                                );
                            }
                            Err(_) => {}
                        }
                    }
                } else {
                    // A transient try-lock miss must not lose an idle update.
                    let _ = ui
                        .as_weak()
                        .upgrade_in_event_loop(|ui| ui.invoke_native_snapshot_ready());
                }
            }
        });
        let weak = ui.as_weak();
        playback.set_snapshot_notifier(Arc::new(move || {
            let _ = weak.upgrade_in_event_loop(|ui| ui.invoke_native_snapshot_ready());
        }));
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
#[path = "desktop_tests.rs"]
mod tests;
