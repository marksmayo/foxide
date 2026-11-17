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
