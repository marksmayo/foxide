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

struct TestPlatform(
    Rc<MinimalSoftwareWindow>,
    std::sync::mpsc::Sender<Box<dyn FnOnce() + Send>>,
);
struct TestProxy(std::sync::mpsc::Sender<Box<dyn FnOnce() + Send>>);
impl slint::platform::EventLoopProxy for TestProxy {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        Ok(())
    }
    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), slint::EventLoopError> {
        self.0
            .send(event)
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }
}
impl Platform for TestPlatform {
    fn new_event_loop_proxy(&self) -> Option<Box<dyn slint::platform::EventLoopProxy>> {
        Some(Box::new(TestProxy(self.1.clone())))
    }
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
    let (event_sender, event_receiver) = std::sync::mpsc::channel();
    slint::platform::set_platform(Box::new(TestPlatform(
        rendered_window.clone(),
        event_sender,
    )))
    .unwrap();
    let ui = PlayerWindow::new().unwrap();
    ui.window().set_size(slint::PhysicalSize::new(1180, 770));
    ui.show().unwrap();
    install_cast(&ui, &Rc::new(RefCell::new(UiState::default())));
    ui.invoke_cast_connect("receiver.example:8009".into(), "invalid".into());
    assert!(
        ui.get_cast_status()
            .contains("literal local receiver address")
    );
    ui.invoke_cast_connect("192.168.1.20:8009".into(), "invalid".into());
    assert!(ui.get_cast_status().contains("64 digit SHA-256"));
    let preferences = Rc::new(RefCell::new(player_settings::Preferences::default()));
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
    let saved = player_settings::Preferences::load(&settings_path).unwrap();
    assert!(saved.compact());
    assert!(saved.light_theme());
    assert_eq!(
        saved.resolve_key("k", true, false, false, false),
        Some(("pause", 0.0))
    );
    ui.invoke_save_binding("Escape".into(), "quit".into());
    assert!(!ui.get_preferences_error().is_empty());
    assert_eq!(
        player_settings::Preferences::load(&settings_path).unwrap(),
        saved
    );
    ui.invoke_toggle_compact();
    assert!(!ui.get_compact_mode());
    assert_eq!(ui.window().size(), normal_size);
    ui.invoke_remove_binding("Ctrl+K".into());
    assert!(!ui.invoke_shortcut("k".into(), true, false, false, false));
    ui.invoke_reset_preferences();
    assert_eq!(
        player_settings::Preferences::load(&settings_path).unwrap(),
        player_settings::Preferences::default()
    );
    assert!(!ui.get_light_theme());
    let chosen_helper = std::env::current_exe().unwrap();
    ui.invoke_save_video_site_helper(chosen_helper.to_str().unwrap().into());
    assert_eq!(
        player_settings::Preferences::load(&settings_path)
            .unwrap()
            .video_site_helper(),
        player_core::external_path(chosen_helper.canonicalize().unwrap()).to_str()
    );
    assert!(ui.get_video_site_notice().contains("Restart"));
    ui.invoke_save_video_site_helper("yt-dlp".into());
    assert!(ui.get_preferences_error().contains("absolute"));
    ui.invoke_save_video_site_helper("".into());
    assert_eq!(
        player_settings::Preferences::load(&settings_path)
            .unwrap()
            .video_site_helper(),
        None
    );
    struct RadioFixture;
    impl player_services::HttpClient for RadioFixture {
        fn get(
            &self,
            url: &player_services::Url,
            cancel: &std::sync::atomic::AtomicBool,
        ) -> Result<Vec<u8>, String> {
            let query = url
                .query_pairs()
                .find(|(key, _)| key == "name")
                .map(|(_, value)| value.into_owned())
                .unwrap_or_default();
            if query == "broken" {
                return Err("Directory unavailable".into());
            }
            if query == "slow" {
                while !cancel.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                return Err("cancelled".into());
            }
            Ok(br#"[{"stationuuid":"abc","name":"Fresh Jazz","url_resolved":"https://stream.example.org/live","country":"NZ","lastcheckok":1}]"#.to_vec())
        }
    }
    let _radio = network_ui::install_radio(
        &ui,
        Arc::new(RadioFixture),
        player_services::RadioDirectory::new("https://radio.example.org").unwrap(),
    );
    let pump_radio = || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while ui.get_radio_busy() && std::time::Instant::now() < deadline {
            if let Ok(event) = event_receiver.recv_timeout(Duration::from_millis(20)) {
                event();
            }
        }
        assert!(
            !ui.get_radio_busy(),
            "Radio request must complete or cancel"
        );
    };
    ui.set_panel("radio".into());
    ui.invoke_radio_search("jazz".into());
    assert!(ui.get_radio_busy());
    pump_radio();
    assert_eq!(ui.get_radio_stations().row_count(), 1);
    capture_customization(&ui, &rendered_window, "radio");
    let played_url = Rc::new(RefCell::new(String::new()));
    let recorded_url = played_url.clone();
    ui.on_open_url(move |url| *recorded_url.borrow_mut() = url.to_string());
    ui.invoke_radio_play("https://stream.example.org/live".into(), "abc".into());
    assert_eq!(
        played_url.borrow().as_str(),
        "https://stream.example.org/live"
    );
    ui.set_panel("radio".into());
    ui.invoke_radio_search("slow".into());
    ui.invoke_cancel_services();
    ui.invoke_radio_search("fresh".into());
    pump_radio();
    assert_eq!(
        ui.get_radio_stations().row_data(0).unwrap().name,
        "Fresh Jazz"
    );
    ui.set_loaded(true);
    let before_radio_status = ui.get_status();
    ui.invoke_radio_search("broken".into());
    pump_radio();
    assert!(ui.get_radio_error().contains("unavailable"));
    assert!(
        ui.get_loaded(),
        "Service errors must not stop active playback"
    );
    assert_eq!(ui.get_status(), before_radio_status);
    ui.set_loaded(false);
    ui.set_panel("".into());
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
    snapshot.command_error = "Unsupported optional renderer control".into();
    refresh(&ui, &state, &snapshot, &mut previous);
    assert!(ui.get_loaded() && state.borrow().loaded);
    assert_eq!(ui.get_status(), "Unsupported optional renderer control");
    snapshot.command_error.clear();
    refresh(&ui, &state, &snapshot, &mut previous);
    assert_eq!(ui.get_status(), "Playing · 1.00×");
    snapshot.has_artwork = true;
    refresh(&ui, &state, &snapshot, &mut previous);
    assert!(ui.get_has_artwork());
    ui.invoke_toggle_compact();
    assert_eq!(ui.get_video_width(), 56.0);
    assert_eq!(ui.get_video_height(), 56.0);
    let mut art = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(8, 8);
    art.make_mut_slice()
        .fill(slint::Rgb8Pixel { r: 255, g: 0, b: 0 });
    ui.set_video_frame(slint::Image::from_rgb8(art));
    let size = ui.window().size();
    let mut pixels = vec![slint::Rgb8Pixel::default(); (size.width * size.height) as usize];
    ui.window().request_redraw();
    rendered_window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, size.width as usize);
    });
    capture_customization(&ui, &rendered_window, "artwork");
    let red_pixels = pixels
        .iter()
        .filter(|p| p.r > 240 && p.g < 10 && p.b < 10)
        .count();
    assert!(
        red_pixels > 2500,
        "Compact mode must actually draw the artwork: {red_pixels} red pixels"
    );
    snapshot.has_artwork = false;
    refresh(&ui, &state, &snapshot, &mut previous);
    assert!(!ui.get_has_artwork());
    assert_eq!(ui.get_video_width(), 1.0);
    pixels.fill(slint::Rgb8Pixel::default());
    ui.window().request_redraw();
    rendered_window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, size.width as usize);
    });
    assert!(
        !pixels.iter().any(|p| p.r > 240 && p.g < 10 && p.b < 10),
        "Artwork from the previous file must disappear"
    );
    ui.invoke_toggle_compact();
    let mut tracks = snapshot.tracks.to_vec();
    tracks.push(player_render::Track {
        id: 2,
        kind: "video".into(),
        label: "Video".into(),
        selected: false,
    });
    snapshot.tracks = tracks.into();
    refresh(&ui, &state, &snapshot, &mut previous);
    assert!(
        !ui.get_has_video(),
        "An unselected video track must not enable frame controls"
    );
    Arc::make_mut(&mut snapshot.tracks)[1].selected = true;
    refresh(&ui, &state, &snapshot, &mut previous);
    assert!(ui.get_has_video());
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
    let controls = Rc::new(RefCell::new(UiState::default()));
    install_controls(&ui, &controls, &None);
    ui.invoke_open_url("file:///not-an-online-stream".into());
    assert!(ui.get_status().contains("HTTP"));
    ui.invoke_open_url("https://example.org/movie.mp4".into());
    assert_eq!(controls.borrow().playlist.len(), 1);
    assert!(ui.get_status().contains("unavailable"));
    ui.invoke_filter_queue("no-match".into());
    assert_eq!(ui.get_queue().row_count(), 0);
    ui.invoke_filter_queue("".into());
    assert_eq!(ui.get_queue().row_count(), 1);
    ui.invoke_files_picked(
        slint::ModelRc::new(slint::VecModel::from(vec![
            "one.mkv".into(),
            "two.mkv".into(),
        ])),
        false,
    );
    assert_eq!(controls.borrow().playlist.len(), 2);
    for (name, value) in [
        ("pause", 0.0),
        ("mute", 0.0),
        ("seek-relative", 5.0),
        ("seek", 50.0),
        ("speed", 1.5),
        ("volume", 75.0),
        ("audio-track", 1.0),
        ("subtitle-track", 1.0),
        ("subtitle-track", 0.0),
        ("frame-step", 0.0),
    ] {
        controls.borrow_mut().confirm_loaded(120.0);
        ui.invoke_action(name.into(), value);
        assert!(
            ui.get_status().contains("unavailable"),
            "{name} must report a missing engine"
        );
    }
    ui.invoke_action("playlist".into(), 0.0);
    assert!(ui.get_queue_visible());
    ui.invoke_action("fullscreen".into(), 0.0);
    assert!(controls.borrow().fullscreen);
    ui.invoke_action("preferences".into(), 0.0);
    assert_eq!(ui.get_panel(), "preferences");
    ui.invoke_action("url".into(), 0.0);
    assert_eq!(ui.get_panel(), "url");
    ui.invoke_action("next".into(), 0.0);
    assert_eq!(controls.borrow().selected, 1);
    ui.invoke_action("previous".into(), 0.0);
    assert_eq!(controls.borrow().selected, 0);
    ui.invoke_action("select".into(), 1.0);
    assert_eq!(controls.borrow().selected, 1);
    ui.invoke_action("stop".into(), 0.0);
    assert!(!controls.borrow().loaded);
    let count = controls.borrow().playlist.len();
    ui.invoke_files_picked(
        slint::ModelRc::new(slint::VecModel::from(vec!["subtitle.srt".into()])),
        true,
    );
    assert_eq!(
        controls.borrow().playlist.len(),
        count,
        "Subtitle loading must not replace the playlist"
    );
    let before = ui.get_status();
    ui.invoke_action("unknown-action".into(), 0.0);
    assert_eq!(ui.get_status(), before);
    controls.borrow_mut().confirm_loaded(120.0);
    ui.set_loaded(true);
    ui.set_has_video(false);
    ui.set_has_artwork(true);
    ui.set_status("Playing audio".into());
    for name in ["frame-step", "frame-back-step"] {
        ui.invoke_action(name.into(), 0.0);
        assert_eq!(
            ui.get_status(),
            "Playing audio",
            "Audio artwork must not enable {name}"
        );
    }
    shader_ui::install(&ui, &None);
    ui.invoke_clear_shaders();
    assert_eq!(ui.get_shader_notice(), "Playback engine unavailable");
    let applied_shaders = Rc::new(RefCell::new(Vec::<Vec<PathBuf>>::new()));
    let recorded_shaders = applied_shaders.clone();
    shader_ui::install_with_apply(
        &ui,
        Rc::new(move |files| {
            recorded_shaders
                .borrow_mut()
                .push(files.iter().map(|file| file.path().to_path_buf()).collect());
            Ok(())
        }),
    );
    let shader_paths =
        |paths: Vec<slint::SharedString>| slint::ModelRc::new(slint::VecModel::from(paths));
    let pump_shaders = || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while (ui.get_shader_notice().starts_with("Choose trusted")
            || ui.get_shader_notice().contains("already open"))
            && std::time::Instant::now() < deadline
        {
            if let Ok(event) = event_receiver.recv_timeout(Duration::from_millis(20)) {
                event();
            }
        }
        assert!(
            !ui.get_shader_notice().starts_with("Choose trusted")
                && !ui.get_shader_notice().contains("already open"),
            "Shader validation must finish"
        );
    };
    ui.invoke_load_shader_files(shader_paths(vec!["relative.glsl".into()]));
    ui.invoke_load_shader_files(shader_paths(vec!["relative.glsl".into()]));
    assert!(ui.get_shader_notice().contains("already open"));
    pump_shaders();
    assert!(ui.get_shader_notice().contains("absolute"));
    assert!(applied_shaders.borrow().is_empty());
    assert!(
        ui.get_loaded(),
        "Shader validation errors must preserve playback"
    );
    ui.invoke_load_shader_files(shader_paths(vec!["relative.glsl".into(); 9]));
    pump_shaders();
    assert!(ui.get_shader_notice().contains("eight"));
    let shader_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../player-render/tests/fixtures/invert,colors;fixture.glsl");
    ui.invoke_load_shader_files(shader_paths(vec![shader_path.to_str().unwrap().into()]));
    pump_shaders();
    assert!(ui.get_shader_notice().contains("Applying 1 shader"));
    assert_eq!(
        applied_shaders.borrow()[0],
        vec![shader_path.canonicalize().unwrap()]
    );
    ui.set_panel("information".into());
    capture_customization(&ui, &rendered_window, "shaders");
    ui.invoke_clear_shaders();
    assert!(applied_shaders.borrow().last().unwrap().is_empty());
    for event in event_receiver.try_iter() {
        event();
    }
    let before = applied_shaders.borrow().len();
    ui.invoke_load_shader_files(shader_paths(vec![shader_path.to_str().unwrap().into()]));
    ui.invoke_clear_shaders();
    event_receiver.recv_timeout(Duration::from_secs(5)).unwrap()();
    assert_eq!(
        applied_shaders.borrow().len(),
        before + 1,
        "Clear must invalidate a pending selection; its late result cannot re-enable shaders"
    );
    assert!(applied_shaders.borrow().last().unwrap().is_empty());
    shader_ui::install_with_apply(&ui, Rc::new(|_| Err("Renderer queue is full".into())));
    ui.invoke_clear_shaders();
    assert_eq!(ui.get_shader_notice(), "Renderer queue is full");
    let redirected = Rc::new(RefCell::new(UiState::default()));
    redirected
        .borrow_mut()
        .dispatch(Action::Open(vec!["https://page.example.org/watch".into()]));
    ui.set_loaded(false);
    let redirected_snapshot = Snapshot {
        path: "https://cdn.example.org/resolved-video.mp4".into(),
        requested_path: "https://page.example.org/watch".into(),
        loaded: true,
        duration: 30.0,
        speed: 1.0,
        ..Default::default()
    };
    refresh(
        &ui,
        &redirected,
        &redirected_snapshot,
        &mut Snapshot::default(),
    );
    assert!(
        ui.get_loaded() && redirected.borrow().loaded,
        "An extracted stream must remain associated with the page the user opened"
    );
    profile_ui::install(&ui, &None);
    ui.invoke_clear_color_profile();
    assert_eq!(ui.get_color_profile_notice(), "Playback engine unavailable");
    let applied_profiles = Rc::new(RefCell::new(Vec::<Option<PathBuf>>::new()));
    let recorded_profiles = applied_profiles.clone();
    profile_ui::install_with_apply(
        &ui,
        Rc::new(move |profile| {
            recorded_profiles
                .borrow_mut()
                .push(profile.map(|profile| profile.path().to_path_buf()));
            Ok(())
        }),
    );
    let pump_profile = || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while (ui
            .get_color_profile_notice()
            .starts_with("Choose a display")
            || ui.get_color_profile_notice().contains("already open"))
            && std::time::Instant::now() < deadline
        {
            if let Ok(event) = event_receiver.recv_timeout(Duration::from_millis(20)) {
                event();
            }
        }
        assert!(
            !ui.get_color_profile_notice()
                .starts_with("Choose a display")
                && !ui.get_color_profile_notice().contains("already open")
        );
    };
    ui.invoke_load_color_profile("relative.icc".into());
    ui.invoke_load_color_profile("relative.icc".into());
    assert!(ui.get_color_profile_notice().contains("already open"));
    pump_profile();
    assert!(ui.get_color_profile_notice().contains("absolute"));
    assert!(applied_profiles.borrow().is_empty() && ui.get_loaded());
    let profile_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../player-render/tests/fixtures/srgb.icc");
    ui.invoke_load_color_profile(profile_path.to_str().unwrap().into());
    pump_profile();
    assert_eq!(
        applied_profiles.borrow()[0],
        Some(profile_path.canonicalize().unwrap())
    );
    ui.set_panel("information".into());
    ui.set_shader_notice("No custom shaders selected".into());
    capture_customization(&ui, &rendered_window, "color-profile");
    ui.invoke_clear_color_profile();
    assert!(applied_profiles.borrow().last().unwrap().is_none());
    for event in event_receiver.try_iter() {
        event();
    }
    let count = applied_profiles.borrow().len();
    ui.invoke_load_color_profile(profile_path.to_str().unwrap().into());
    ui.invoke_clear_color_profile();
    event_receiver.recv_timeout(Duration::from_secs(5)).unwrap()();
    assert_eq!(
        applied_profiles.borrow().len(),
        count + 1,
        "Clearing a profile must invalidate late selections"
    );
    profile_ui::install_with_apply(
        &ui,
        Rc::new(|_| Err("Profile command queue is full".into())),
    );
    ui.invoke_clear_color_profile();
    assert_eq!(
        ui.get_color_profile_notice(),
        "Profile command queue is full"
    );
}
