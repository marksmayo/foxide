use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Open(Vec<PathBuf>),
    FrameStep(bool),
    SecondarySubtitle(u32),
    Rotation(u16),
    ReplayGain(u8),
    Deband(bool),
    Equalizer(u8),
    Flip(bool),
    Seek(f64),
    TogglePause,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Engine(Vec<String>),
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
        Some(match name {
            "frame-step" => Self::FrameStep(false),
            "frame-back-step" => Self::FrameStep(true),
            "secondary-subtitle" if value.is_finite() && value >= 0.0 && value <= u32::MAX as f64 && value.fract() == 0.0 => Self::SecondarySubtitle(value as u32),
            "rotation" if [0.0, 90.0, 180.0, 270.0].contains(&value) => Self::Rotation(value as u16),
            "flip-vertical" => Self::Flip(true),
            "flip-horizontal" => Self::Flip(false),
            "replaygain" if [0.0, 1.0, 2.0].contains(&value) => Self::ReplayGain(value as u8),
            "deband" if [0.0, 1.0].contains(&value) => Self::Deband(value == 1.0),
            "equalizer" if [0.0, 1.0, 2.0].contains(&value) => Self::Equalizer(value as u8),
            _ => return None,
        })
    }
}

impl UiState {
    /// Stable queue indices keep filtered selection attached to the original file.
    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.playlist.iter().enumerate()
            .filter(|(_, path)| query.is_empty() || path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_lowercase().contains(&query))
            .map(|(index, _)| index).collect()
    }

    pub fn confirm_loaded(&mut self, duration: f64) {
        self.loaded = true;
        self.duration = if duration.is_finite() {
            duration.max(0.0)
        } else {
            0.0
        };
    }

    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Equalizer(preset) if self.loaded && preset <= 2 => vec![Effect::Engine(vec!["af".into(), "set".into(), ["", "lavfi=[equalizer=f=100:t=o:w=2:g=6]", "lavfi=[equalizer=f=3000:t=o:w=2:g=4]"][preset as usize].into()])],
            Action::Deband(enabled) if self.loaded => vec![Effect::Engine(vec!["set".into(), "deband".into(), if enabled { "yes" } else { "no" }.into()])],
            Action::ReplayGain(mode) if self.loaded && mode <= 2 => vec![Effect::Engine(vec!["set".into(), "replaygain".into(), ["no", "track", "album"][mode as usize].into()])],
            Action::Rotation(angle) if self.loaded && [0, 90, 180, 270].contains(&angle) => vec![Effect::Engine(vec!["set".into(), "video-rotate".into(), angle.to_string()])],
            Action::Flip(vertical) if self.loaded => vec![Effect::Engine(vec!["vf".into(), "toggle".into(), if vertical { "@foxide-vflip:vflip" } else { "@foxide-hflip:hflip" }.into()])],
            Action::SecondarySubtitle(id) if self.loaded => vec![Effect::Engine(vec!["set".into(), "secondary-sid".into(), if id == 0 { "no".into() } else { id.to_string() }])],
            Action::FrameStep(backward) if self.loaded => vec![Effect::Engine(vec![if backward { "frame-back-step" } else { "frame-step" }.into()])],
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
            _ => Vec::new(),
        }
    }

    fn select(&mut self, index: usize) -> Vec<Effect> {
        self.selected = index;
        self.loaded = false;
        vec![Effect::Load(self.playlist[index].clone())]
    }
}
