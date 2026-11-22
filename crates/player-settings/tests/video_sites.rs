use player_settings::Preferences;
#[test]
fn helper_choice_survives_restart_and_can_be_disabled_without_changing_other_preferences() {
    let mut settings = Preferences::default();
    let original = settings.clone();
    assert_eq!(settings.video_site_helper(), None);
    let path = std::env::current_exe().unwrap();
    let path = path.to_str().unwrap();
    settings.set_video_site_helper(Some(path)).unwrap();
    let restored = Preferences::from_json(&settings.to_json()).unwrap();
    assert_eq!(restored.video_site_helper(), Some(path));
    let before = settings.clone();
    for invalid in ["yt-dlp", "", "bad\npath"] {
        assert!(settings.set_video_site_helper(Some(invalid)).is_err());
        assert_eq!(settings, before);
    }
    settings.set_video_site_helper(None).unwrap();
    assert_eq!(settings, original);
    let mut value: serde_json::Value = serde_json::from_str(&settings.to_json()).unwrap();
    value["video_site_helper"] = serde_json::json!(true);
    assert!(Preferences::from_json(&value.to_string()).is_err());
}
