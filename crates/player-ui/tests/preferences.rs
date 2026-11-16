use player_ui::Preferences;

#[test]
fn keyboard_bindings_can_be_edited_saved_and_loaded_without_changing_defaults() {
    let mut prefs = Preferences::default();
    assert_eq!(
        prefs.resolve_key("Space", false, false, false, false),
        Some(("pause", 0.0))
    );
    prefs.bind("Ctrl+K", "pause").unwrap();
    prefs.unbind("Space").unwrap();
    let saved = prefs.to_json();
    let restored = Preferences::from_json(&saved).unwrap();
    assert_eq!(
        restored.resolve_key("k", true, false, false, false),
        Some(("pause", 0.0))
    );
    assert_eq!(
        restored.resolve_key("Space", false, false, false, false),
        None
    );
    assert_eq!(
        restored.resolve_key("ArrowLeft", false, false, false, false),
        Some(("seek-relative", -5.0))
    );
    assert_eq!(
        Preferences::default().resolve_key("Space", false, false, false, false),
        Some(("pause", 0.0))
    );
}

#[test]
fn preferences_persist_across_restarts_and_invalid_edits_do_not_replace_the_file() {
    let path = std::env::temp_dir().join(format!(
        "foxide-prefs-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut prefs = Preferences::default();
    prefs.bind("Alt+K", "pause").unwrap();
    prefs.save(&path).unwrap();
    let mut loaded = Preferences::load(&path).unwrap();
    assert_eq!(loaded, prefs);
    loaded.unbind("Alt+K").unwrap();
    loaded.save(&path).unwrap();
    assert_eq!(Preferences::load(&path).unwrap(), loaded);
    assert!(loaded.bind("Escape", "quit").is_err());
    assert!(loaded.bind("Ctrl+O", "quit").is_err());
    assert!(loaded.bind("Ctrl+Ctrl+Z", "pause").is_err());
    assert!(loaded.bind("x", "run-shell").is_err());
    assert_eq!(Preferences::load(&path).unwrap(), loaded);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn custom_appearance_survives_restart_and_rejects_invalid_colors() {
    let mut prefs = Preferences::default();
    prefs.set_appearance(true, "#3178ce").unwrap();
    let restored = Preferences::from_json(&prefs.to_json()).unwrap();
    assert!(restored.light_theme());
    assert_eq!(restored.accent_rgb(), 0x3178ce);
    assert!(prefs.set_appearance(false, "not-a-color").is_err());
    assert!(prefs.light_theme());
    assert_eq!(prefs.accent_rgb(), 0x3178ce);
}

#[test]
fn compact_music_view_is_an_independent_persistent_preference() {
    let mut prefs = Preferences::default();
    assert!(!prefs.compact());
    prefs.set_compact(true);
    let restored = Preferences::from_json(&prefs.to_json()).unwrap();
    assert!(restored.compact());
    assert_eq!(
        restored.resolve_key("Space", false, false, false, false),
        Some(("pause", 0.0))
    );
    prefs.set_compact(false);
    assert!(!Preferences::from_json(&prefs.to_json()).unwrap().compact());
}
