use player_core::PlaybackOptions;

#[test]
fn speed_control_keeps_hardware_policy_and_rejects_invalid_rates() {
    let mut options = PlaybackOptions {
        speed: 2.0,
        ..Default::default()
    };
    let args = options.mpv_args().unwrap();
    assert!(args.iter().any(|s| s == "--speed=2"));
    assert!(args.iter().any(|s| s == "--hwdec=auto-safe"));
    for invalid in [0.0, 0.24, 4.01, f64::NAN, f64::INFINITY] {
        options.speed = invalid;
        assert!(options.mpv_args().is_err());
    }
}

#[test]
fn folder_queue_uses_episode_order_and_ignores_non_media_and_subdirectories() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../results/local");
    std::fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!(
        "queue-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    for name in [
        "Episode10.MKV",
        "Episode2.mp4",
        "Episode01.mp4",
        "subtitles.srt",
        "cover.png",
    ] {
        std::fs::write(directory.join(name), []).unwrap();
    }
    std::fs::create_dir(directory.join("Episode3.mp4")).unwrap();
    let queue = player_core::folder_queue(&directory).unwrap();
    assert_eq!(
        queue
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect::<Vec<_>>(),
        ["Episode01.mp4", "Episode2.mp4", "Episode10.MKV"]
    );
    let resolved = directory.canonicalize().unwrap();
    assert!(resolved.starts_with(root.canonicalize().unwrap()));
    std::fs::remove_dir_all(&resolved).unwrap();
    assert!(player_core::folder_queue(&resolved).is_err());
}

#[test]
fn resume_chapters_and_minimal_view_have_explicit_engine_settings() {
    let options = PlaybackOptions {
        resume_dir: Some("state with spaces".into()),
        minimal: true,
        on_top: true,
        chapter: Some(2),
        ab_loop: Some((5.0, 10.0)),
        loop_playlist: true,
        ..Default::default()
    };
    let args = options.mpv_args().unwrap();
    for expected in [
        "--save-position-on-quit=yes",
        "--watch-later-directory=state with spaces",
        "--border=no",
        "--ontop=yes",
        "--start=#2",
        "--ab-loop-a=5",
        "--ab-loop-b=10",
        "--loop-playlist=inf",
    ] {
        assert!(args.iter().any(|s| s == expected), "missing {expected}");
    }
    let invalid = PlaybackOptions {
        start: Some(2.0),
        chapter: Some(1),
        ..Default::default()
    };
    assert!(invalid.mpv_args().is_err());
    for range in [(10.0, 5.0), (-1.0, 5.0), (0.0, f64::NAN)] {
        assert!(
            PlaybackOptions {
                ab_loop: Some(range),
                ..Default::default()
            }
            .mpv_args()
            .is_err()
        );
    }
}

#[test]
fn picture_controls_and_start_position_are_validated_before_launch() {
    let mut options = PlaybackOptions {
        brightness: 10,
        contrast: -20,
        gamma: 5,
        deinterlace: true,
        tone_map: true,
        start: Some(12.5),
        ..Default::default()
    };
    let args = options.mpv_args().unwrap();
    for expected in [
        "--brightness=10",
        "--contrast=-20",
        "--gamma=5",
        "--deinterlace=yes",
        "--tone-mapping=bt.2390",
        "--target-trc=bt.1886",
        "--start=12.5",
    ] {
        assert!(args.iter().any(|s| s == expected), "missing {expected}");
    }
    options.brightness = 101;
    assert!(options.mpv_args().is_err());
    options.brightness = 0;
    options.start = Some(-1.0);
    assert!(options.mpv_args().is_err());
}

#[test]
fn tracks_and_external_subtitles_keep_signed_sync_offsets() {
    let mut options = PlaybackOptions {
        audio_track: Some(2),
        subtitle_track: Some(3),
        subtitle_files: vec!["subtitles with spaces.ass".into()],
        audio_delay: -0.25,
        subtitle_delay: 1.5,
        ..Default::default()
    };
    let args = options.mpv_args().unwrap();
    for expected in [
        "--aid=2",
        "--sid=3",
        "--sub-file=subtitles with spaces.ass",
        "--audio-delay=-0.25",
        "--sub-delay=1.5",
    ] {
        assert!(args.iter().any(|s| s == expected), "missing {expected}");
    }
    options.audio_delay = f64::NAN;
    assert!(options.mpv_args().is_err());
    options.audio_delay = 0.0;
    options.subtitle_delay = f64::INFINITY;
    assert!(options.mpv_args().is_err());
    options.subtitle_delay = 0.0;
    options.audio_track = Some(0);
    assert!(options.mpv_args().is_err());
}
