use std::path::PathBuf;

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
    use super::{Action, Effect, UiState};

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
}
