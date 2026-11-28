use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Open(Vec<PathBuf>),
    Seek(f64),
    TogglePause,
    SetPaused(bool),
    ToggleMute,
    SetSpeed(f64),
    Next,
    Previous,
    Select(usize),
    SeekAbsolute(f64),
    Volume(f64),
    AudioTrack(u32),
    SubtitleTrack(u32),
    TogglePlaylist,
    ToggleFullscreen,
    Stop,
    Quit,
    Playback(PlaybackControl),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackControl {
    FrameStep(bool),
    SecondarySubtitle(u32),
    Rotation(u16),
    ReplayGain(u8),
    Deband(bool),
    Equalizer(u8),
    Flip(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Load(PathBuf),
    Pause(bool),
    Mute(bool),
    Seek(f64),
    Speed(f64),
    SeekAbsolute(f64),
    Volume(f64),
    AudioTrack(u32),
    SubtitleTrack(u32),
    Stop,
    Quit,
    Engine(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscKind {
    Dvd,
    BluRay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscNavigationAction {
    Menu,
    Popup,
    Up,
    Down,
    Left,
    Right,
    Select,
    Previous,
}

impl DiscNavigationAction {
    pub fn from_ui_action(name: &str) -> Option<Self> {
        Some(match name {
            "disc-menu" => Self::Menu,
            "disc-popup" => Self::Popup,
            "disc-up" => Self::Up,
            "disc-down" => Self::Down,
            "disc-left" => Self::Left,
            "disc-right" => Self::Right,
            "disc-select" => Self::Select,
            "disc-back" => Self::Previous,
            _ => return None,
        })
    }

    fn command(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Popup => "popup",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Select => "select",
            Self::Previous => "prev",
        }
    }
}

impl DiscKind {
    pub fn validate_source(self, source: &Path) -> Result<(), &'static str> {
        if source.is_dir() {
            return self.validate_folder(source);
        }
        if !source.is_file() {
            return Err("Choose an existing disc folder or ISO image");
        }
        if !source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("iso"))
        {
            return Err("Choose a DVD or Blu-ray folder or .iso image");
        }

        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(source).map_err(|_| "Could not read the disc image")?;
        let mut iso9660 = [0_u8; 2_048];
        let has_iso9660 = file
            .seek(SeekFrom::Start(16 * 2_048))
            .and_then(|_| file.read_exact(&mut iso9660))
            .is_ok()
            && iso9660[0] == 1
            && iso9660[1..6] == *b"CD001"
            && iso9660[6] == 1;

        let mut udf_anchor = [0_u8; 16];
        let has_udf_anchor = file
            .seek(SeekFrom::Start(256 * 2_048))
            .and_then(|_| file.read_exact(&mut udf_anchor))
            .is_ok()
            && u16::from_le_bytes([udf_anchor[0], udf_anchor[1]]) == 2
            && u32::from_le_bytes(udf_anchor[12..16].try_into().unwrap()) == 256;
        if has_iso9660 || has_udf_anchor {
            Ok(())
        } else {
            Err("This .iso file has no supported ISO9660 or UDF volume descriptor")
        }
    }

    pub fn validate_folder(self, device: &Path) -> Result<(), &'static str> {
        if !device.is_dir() {
            return Err("Choose an existing disc folder");
        }
        use std::io::Read;
        let has_signature = |path: &Path, signature: &[u8]| {
            let Ok(mut file) = std::fs::File::open(path) else {
                return false;
            };
            let mut prefix = vec![0; signature.len()];
            file.read_exact(&mut prefix).is_ok() && prefix == signature
        };
        let has_dvd_video = [
            device.join("VIDEO_TS/VIDEO_TS.IFO"),
            device.join("VIDEO_TS.IFO"),
        ]
        .iter()
        .any(|path| has_signature(path, b"DVDVIDEO-VMG"));
        let has_bluray_header = |path: &Path| {
            let Ok(mut file) = std::fs::File::open(path) else {
                return false;
            };
            let mut header = [0; 8];
            file.read_exact(&mut header).is_ok()
                && &header[..4] == b"INDX"
                && header[4..].iter().all(u8::is_ascii_digit)
        };
        let has_bluray_index = [device.join("BDMV/index.bdmv"), device.join("index.bdmv")]
            .iter()
            .any(|path| has_bluray_header(path));
        match self {
            Self::Dvd if has_dvd_video => Ok(()),
            Self::BluRay if has_bluray_index => Ok(()),
            Self::Dvd => Err("This folder does not contain a DVD VIDEO_TS.IFO structure"),
            Self::BluRay => Err("This folder does not contain a Blu-ray BDMV/index.bdmv structure"),
        }
    }
}

#[derive(Debug, Default)]
pub struct UiState {
    pub playlist: Vec<PathBuf>,
    pub loaded: bool,
    pub paused: bool,
    pub muted: bool,
    pub duration: f64,
    pub selected: usize,
    pub playlist_visible: bool,
    pub fullscreen: bool,
}

impl Action {
    /// Map only supported desktop controls; no arbitrary engine command input.
    pub fn control(name: &str, value: f64) -> Option<Self> {
        let control = match name {
            "frame-step" => PlaybackControl::FrameStep(false),
            "frame-back-step" => PlaybackControl::FrameStep(true),
            "secondary-subtitle"
                if value.is_finite()
                    && value >= 0.0
                    && value <= u32::MAX as f64
                    && value.fract() == 0.0 =>
            {
                PlaybackControl::SecondarySubtitle(value as u32)
            }
            "rotation" if [0.0, 90.0, 180.0, 270.0].contains(&value) => {
                PlaybackControl::Rotation(value as u16)
            }
            "flip-vertical" => PlaybackControl::Flip(true),
            "flip-horizontal" => PlaybackControl::Flip(false),
            "replaygain" if [0.0, 1.0, 2.0].contains(&value) => {
                PlaybackControl::ReplayGain(value as u8)
            }
            "deband" if [0.0, 1.0].contains(&value) => PlaybackControl::Deband(value == 1.0),
            "equalizer" if [0.0, 1.0, 2.0].contains(&value) => {
                PlaybackControl::Equalizer(value as u8)
            }
            _ => return None,
        };
        Some(Self::Playback(control))
    }
}

impl PlaybackControl {
    // Keep optional feature command construction out of the existing hot dispatch path.
    #[inline(never)]
    fn command(self) -> Option<Vec<String>> {
        match self {
            Self::Equalizer(preset) if preset <= 2 => Some(vec![
                "af".into(),
                "set".into(),
                [
                    "",
                    "lavfi=[equalizer=f=100:t=o:w=2:g=6]",
                    "lavfi=[equalizer=f=3000:t=o:w=2:g=4]",
                ][preset as usize]
                    .into(),
            ]),
            Self::Deband(enabled) => Some(vec![
                "set".into(),
                "deband".into(),
                if enabled { "yes" } else { "no" }.into(),
            ]),
            Self::ReplayGain(mode) if mode <= 2 => Some(vec![
                "set".into(),
                "replaygain".into(),
                ["no", "track", "album"][mode as usize].into(),
            ]),
            Self::Rotation(angle) if [0, 90, 180, 270].contains(&angle) => {
                Some(vec!["set".into(), "video-rotate".into(), angle.to_string()])
            }
            Self::Flip(vertical) => Some(vec![
                "vf".into(),
                "toggle".into(),
                if vertical {
                    "@foxide-vflip:vflip"
                } else {
                    "@foxide-hflip:hflip"
                }
                .into(),
            ]),
            Self::SecondarySubtitle(id) => Some(vec![
                "set".into(),
                "secondary-sid".into(),
                if id == 0 { "no".into() } else { id.to_string() },
            ]),
            Self::FrameStep(backward) => Some(vec![
                if backward {
                    "frame-back-step"
                } else {
                    "frame-step"
                }
                .into(),
            ]),
            _ => None,
        }
    }
}

impl UiState {
    pub fn open_disc(&mut self, kind: DiscKind, device: PathBuf) -> Vec<Effect> {
        self.playlist = vec![device.clone()];
        self.selected = 0;
        self.loaded = false;
        let (device_option, protocol) = match kind {
            DiscKind::Dvd => ("dvd-device", "dvd://"),
            DiscKind::BluRay => ("bluray-device", "bd://"),
        };
        vec![
            Effect::Engine(vec![
                "set".into(),
                device_option.into(),
                device.to_string_lossy().into_owned(),
            ]),
            Effect::Engine(vec!["set".into(), "disc-menu".into(), "yes".into()]),
            Effect::Engine(vec!["loadfile".into(), protocol.into(), "replace".into()]),
        ]
    }

    pub fn disc_navigation(&self, action: DiscNavigationAction) -> Vec<Effect> {
        if !self.loaded {
            return Vec::new();
        }
        vec![Effect::Engine(vec![
            "discnav".into(),
            action.command().into(),
        ])]
    }

    /// Stable queue indices keep filtered selection attached to the original file.
    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.playlist
            .iter()
            .enumerate()
            .filter(|(_, path)| {
                query.is_empty()
                    || path
                        .file_name()
                        .unwrap_or(path.as_os_str())
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    #[inline]
    pub fn confirm_loaded(&mut self, duration: f64) {
        self.loaded = true;
        self.duration = if duration.is_finite() {
            duration.max(0.0)
        } else {
            0.0
        };
    }

    #[inline]
    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Open(files) => {
                self.playlist = files;
                self.selected = 0;
                self.loaded = false;
                self.playlist
                    .first()
                    .cloned()
                    .map(Effect::Load)
                    .into_iter()
                    .collect()
            }
            Action::TogglePause if self.loaded => vec![Effect::Pause(!self.paused)],
            Action::SetPaused(paused) if self.loaded => {
                vec![Effect::Pause(paused)]
            }
            Action::ToggleMute => vec![Effect::Mute(!self.muted)],
            Action::Seek(value) if self.loaded && value.is_finite() => vec![Effect::Seek(value)],
            Action::SetSpeed(value)
                if self.loaded && value.is_finite() && (0.25..=4.0).contains(&value) =>
            {
                vec![Effect::Speed(value)]
            }
            Action::Next if self.selected + 1 < self.playlist.len() => {
                self.select(self.selected + 1)
            }
            Action::Previous if self.selected > 0 => self.select(self.selected - 1),
            Action::Select(index) if index < self.playlist.len() => self.select(index),
            Action::SeekAbsolute(value) if self.loaded && value.is_finite() => {
                vec![Effect::SeekAbsolute(value.clamp(0.0, self.duration))]
            }
            Action::Volume(value) if value.is_finite() && (0.0..=100.0).contains(&value) => {
                vec![Effect::Volume(value)]
            }
            Action::AudioTrack(id) if self.loaded && id > 0 => vec![Effect::AudioTrack(id)],
            Action::SubtitleTrack(id) if self.loaded => vec![Effect::SubtitleTrack(id)],
            Action::TogglePlaylist => {
                self.playlist_visible = !self.playlist_visible;
                Vec::new()
            }
            Action::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                Vec::new()
            }
            Action::Stop => {
                self.loaded = false;
                self.paused = false;
                vec![Effect::Stop]
            }
            Action::Quit => vec![Effect::Quit],
            Action::Playback(control) if self.loaded => {
                control.command().map(Effect::Engine).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    fn select(&mut self, index: usize) -> Vec<Effect> {
        self.selected = index;
        self.loaded = false;
        vec![Effect::Load(self.playlist[index].clone())]
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, DiscKind, DiscNavigationAction, Effect, UiState};
    use std::path::PathBuf;

    #[test]
    fn system_media_play_and_pause_wait_for_engine_confirmation() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);
        state.paused = true;

        assert_eq!(
            state.dispatch(Action::SetPaused(false)),
            vec![Effect::Pause(false)]
        );
        assert!(state.paused);

        state.paused = false;
        assert_eq!(
            state.dispatch(Action::SetPaused(true)),
            vec![Effect::Pause(true)]
        );
        assert!(!state.paused);

        state.loaded = false;
        assert!(state.dispatch(Action::SetPaused(false)).is_empty());
    }

    #[test]
    fn opening_a_disc_folder_selects_the_device_and_loads_the_disc_protocol() {
        let cases = [
            (DiscKind::Dvd, "dvd-device", "dvd://"),
            (DiscKind::BluRay, "bluray-device", "bd://"),
        ];

        for (kind, option, protocol) in cases {
            let device = PathBuf::from("/media/Disc One");
            let mut state = UiState::default();
            state.confirm_loaded(120.0);

            let effects = state.open_disc(kind, device.clone());

            assert_eq!(state.playlist, vec![device.clone()]);
            assert_eq!(state.selected, 0);
            assert!(!state.loaded);
            assert_eq!(
                effects,
                vec![
                    Effect::Engine(vec![
                        "set".into(),
                        option.into(),
                        device.to_string_lossy().into_owned(),
                    ]),
                    Effect::Engine(vec!["set".into(), "disc-menu".into(), "yes".into()]),
                    Effect::Engine(vec!["loadfile".into(), protocol.into(), "replace".into(),]),
                ]
            );
        }
    }

    #[test]
    fn disc_folder_validation_requires_the_matching_structure() {
        let root = std::env::temp_dir().join(format!(
            "foxide-disc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dvd = root.join("dvd");
        let bluray = root.join("bluray");
        std::fs::create_dir_all(dvd.join("VIDEO_TS")).unwrap();
        std::fs::write(dvd.join("VIDEO_TS/VIDEO_TS.IFO"), b"DVDVIDEO-VMG").unwrap();
        std::fs::create_dir_all(bluray.join("BDMV")).unwrap();
        std::fs::write(bluray.join("BDMV/index.bdmv"), b"INDX0200").unwrap();

        assert!(DiscKind::Dvd.validate_folder(&dvd).is_ok());
        assert!(DiscKind::BluRay.validate_folder(&bluray).is_ok());
        assert!(DiscKind::Dvd.validate_folder(&bluray).is_err());
        assert!(DiscKind::BluRay.validate_folder(&dvd).is_err());
        assert!(
            DiscKind::Dvd
                .validate_folder(&root.join("missing"))
                .is_err()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disc_folder_validation_rejects_empty_structure_markers() {
        let root = std::env::temp_dir().join(format!(
            "foxide-disc-empty-markers-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dvd = root.join("dvd/VIDEO_TS");
        let bluray = root.join("bluray/BDMV");
        std::fs::create_dir_all(&dvd).unwrap();
        std::fs::create_dir_all(&bluray).unwrap();
        std::fs::File::create(dvd.join("VIDEO_TS.IFO")).unwrap();
        std::fs::File::create(bluray.join("index.bdmv")).unwrap();

        assert!(DiscKind::Dvd.validate_folder(&root.join("dvd")).is_err());
        assert!(
            DiscKind::BluRay
                .validate_folder(&root.join("bluray"))
                .is_err()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disc_folder_validation_rejects_truncated_bluray_index_headers() {
        let root = std::env::temp_dir().join(format!(
            "foxide-disc-truncated-index-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bluray = root.join("bluray/BDMV");
        std::fs::create_dir_all(&bluray).unwrap();
        std::fs::write(bluray.join("index.bdmv"), b"INDX").unwrap();

        assert!(
            DiscKind::BluRay
                .validate_folder(&root.join("bluray"))
                .is_err()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disc_iso_validation_requires_a_recognized_volume_descriptor() {
        use std::io::{Seek, SeekFrom, Write};

        let root = std::env::temp_dir().join(format!(
            "foxide-disc-iso-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let iso9660 = root.join("dvd.ISO");
        let mut file = std::fs::File::create(&iso9660).unwrap();
        file.set_len(16 * 2_048 + 2_048).unwrap();
        file.seek(SeekFrom::Start(16 * 2_048)).unwrap();
        file.write_all(&[1, b'C', b'D', b'0', b'0', b'1', 1])
            .unwrap();
        assert!(DiscKind::Dvd.validate_source(&iso9660).is_ok());

        let udf = root.join("bluray.iso");
        let mut file = std::fs::File::create(&udf).unwrap();
        file.set_len(256 * 2_048 + 16).unwrap();
        file.seek(SeekFrom::Start(256 * 2_048)).unwrap();
        let mut anchor = [0_u8; 16];
        anchor[0..2].copy_from_slice(&2_u16.to_le_bytes());
        anchor[12..16].copy_from_slice(&256_u32.to_le_bytes());
        file.write_all(&anchor).unwrap();
        assert!(DiscKind::BluRay.validate_source(&udf).is_ok());

        let malformed = root.join("broken.iso");
        std::fs::write(&malformed, b"not a disc image").unwrap();
        assert!(DiscKind::Dvd.validate_source(&malformed).is_err());
        assert!(
            DiscKind::Dvd
                .validate_source(&root.join("missing.iso"))
                .is_err()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disc_iso_validation_rejects_a_truncated_iso9660_descriptor() {
        use std::io::{Seek, SeekFrom, Write};

        let path = std::env::temp_dir().join(format!(
            "foxide-disc-truncated-iso-{}-{}.iso",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut file = std::fs::File::create(&path).unwrap();
        file.set_len(16 * 2_048 + 7).unwrap();
        file.seek(SeekFrom::Start(16 * 2_048)).unwrap();
        file.write_all(&[1, b'C', b'D', b'0', b'0', b'1', 1])
            .unwrap();
        drop(file);

        assert!(DiscKind::Dvd.validate_source(&path).is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn disc_navigation_controls_map_only_to_supported_discnav_commands() {
        let cases = [
            (DiscNavigationAction::Menu, "menu", "disc-menu"),
            (DiscNavigationAction::Popup, "popup", "disc-popup"),
            (DiscNavigationAction::Up, "up", "disc-up"),
            (DiscNavigationAction::Down, "down", "disc-down"),
            (DiscNavigationAction::Left, "left", "disc-left"),
            (DiscNavigationAction::Right, "right", "disc-right"),
            (DiscNavigationAction::Select, "select", "disc-select"),
            (DiscNavigationAction::Previous, "prev", "disc-back"),
        ];

        let mut state = UiState::default();
        for (action, command, ui_name) in cases {
            assert_eq!(DiscNavigationAction::from_ui_action(ui_name), Some(action));
            state.confirm_loaded(120.0);
            assert_eq!(
                state.disc_navigation(action),
                vec![Effect::Engine(vec!["discnav".into(), command.into()])]
            );
        }
        state.loaded = false;
        assert!(state.disc_navigation(DiscNavigationAction::Menu).is_empty());
        assert_eq!(DiscNavigationAction::from_ui_action("disc-exec"), None);
    }
}
