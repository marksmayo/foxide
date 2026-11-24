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
    Stereo3d(u8),
    Panorama360(u8),
    PanoramaYaw(f64),
    PanoramaPitch(f64),
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
    pub panorama_angles: Option<(f64, f64)>,
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
            "stereo-3d"
                if value.is_finite() && (0.0..=6.0).contains(&value) && value.fract() == 0.0 =>
            {
                PlaybackControl::Stereo3d(value as u8)
            }
            "panorama-360"
                if value.is_finite() && (0.0..=6.0).contains(&value) && value.fract() == 0.0 =>
            {
                PlaybackControl::Panorama360(value as u8)
            }
            "panorama-yaw" if value.is_finite() && (-180.0..=180.0).contains(&value) => {
                PlaybackControl::PanoramaYaw(value)
            }
            "panorama-pitch" if value.is_finite() && (-180.0..=180.0).contains(&value) => {
                PlaybackControl::PanoramaPitch(value)
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
            Self::Stereo3d(0) => Some(vec![
                "vf".into(),
                "remove".into(),
                "@foxide-stereo3d".into(),
            ]),
            Self::Stereo3d(mode @ 1..=6) => {
                let (input, output) = match mode {
                    1 => ("sbsl", "arcd"),
                    2 => ("sbsl", "abl"),
                    3 => ("sbsr", "arcd"),
                    4 => ("sbsr", "sbsl"),
                    5 => ("abl", "arcd"),
                    6 => ("abl", "sbsl"),
                    _ => unreachable!(),
                };
                Some(vec![
                    "vf".into(),
                    "add".into(),
                    format!("@foxide-stereo3d:lavfi=[stereo3d=in={input}:out={output}]"),
                ])
            }
            Self::Panorama360(0) => Some(vec![
                "vf".into(),
                "remove".into(),
                "@foxide-panorama360".into(),
            ]),
            Self::Panorama360(mode @ 1..=6) => {
                let (yaw, pitch) = match mode {
                    1 => (0, 0),
                    2 => (-90, 0),
                    3 => (90, 0),
                    4 => (180, 0),
                    5 => (0, 45),
                    6 => (0, -45),
                    _ => unreachable!(),
                };
                Some(vec![
                    "vf".into(),
                    "add".into(),
                    format!(
                        "@foxide-panorama360:lavfi=[v360=input=e:output=flat:w=1280:h=720:h_fov=90:v_fov=60:yaw={yaw}:pitch={pitch}]"
                    ),
                ])
            }
            Self::PanoramaYaw(_) | Self::PanoramaPitch(_) => None,
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
                self.panorama_angles = None;
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
                self.panorama_angles = None;
                self.loaded = false;
                self.paused = false;
                vec![Effect::Stop]
            }
            Action::Quit => vec![Effect::Quit],
            Action::Playback(
                control @ (PlaybackControl::Panorama360(_)
                | PlaybackControl::PanoramaYaw(_)
                | PlaybackControl::PanoramaPitch(_)),
            ) if self.loaded => self.panorama(control),
            Action::Playback(control) if self.loaded => {
                control.command().map(Effect::Engine).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    fn select(&mut self, index: usize) -> Vec<Effect> {
        self.panorama_angles = None;
        self.selected = index;
        self.loaded = false;
        vec![Effect::Load(self.playlist[index].clone())]
    }
    // Keep the stateful panorama path out of the common playback-action hot path.
    #[inline(never)]
    fn panorama(&mut self, control: PlaybackControl) -> Vec<Effect> {
        match control {
            PlaybackControl::Panorama360(mode) => {
                let angles = match mode {
                    0 => None,
                    1 => Some((0.0, 0.0)),
                    2 => Some((-90.0, 0.0)),
                    3 => Some((90.0, 0.0)),
                    4 => Some((180.0, 0.0)),
                    5 => Some((0.0, 45.0)),
                    6 => Some((0.0, -45.0)),
                    _ => return Vec::new(),
                };
                self.panorama_angles = angles;
                PlaybackControl::Panorama360(mode)
                    .command()
                    .map(Effect::Engine)
                    .into_iter()
                    .collect()
            }
            PlaybackControl::PanoramaYaw(delta) => {
                let Some((yaw, pitch)) = self.panorama_angles else {
                    return Vec::new();
                };
                let yaw = (yaw + delta + 180.0).rem_euclid(360.0) - 180.0;
                self.panorama_angles = Some((yaw, pitch));
                vec![Effect::Engine(vec![
                    "vf-command".into(),
                    "@foxide-panorama360".into(),
                    "yaw".into(),
                    yaw.to_string(),
                    "v360".into(),
                ])]
            }
            PlaybackControl::PanoramaPitch(delta) => {
                let Some((yaw, pitch)) = self.panorama_angles else {
                    return Vec::new();
                };
                let pitch = (pitch + delta).clamp(-89.0, 89.0);
                self.panorama_angles = Some((yaw, pitch));
                vec![Effect::Engine(vec![
                    "vf-command".into(),
                    "@foxide-panorama360".into(),
                    "pitch".into(),
                    pitch.to_string(),
                    "v360".into(),
                ])]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Effect, UiState};

    #[test]
    fn stereo_3d_control_adds_a_labeled_output_filter_without_replacing_user_filters() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);

        assert_eq!(
            state.dispatch(Action::control("stereo-3d", 1.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf".into(),
                "add".into(),
                "@foxide-stereo3d:lavfi=[stereo3d=in=sbsl:out=arcd]".into(),
            ])]
        );
    }

    #[test]
    fn panorama_360_front_view_adds_a_labeled_equirectangular_projection_filter() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);

        assert_eq!(
            state.dispatch(Action::control("panorama-360", 1.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf".into(),
                "add".into(),
                "@foxide-panorama360:lavfi=[v360=input=e:output=flat:w=1280:h=720:h_fov=90:v_fov=60:yaw=0:pitch=0]".into(),
            ])]
        );
    }

    #[test]
    fn panorama_360_presets_replace_the_view_and_off_removes_only_its_filter() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);
        let cases = [
            (2.0, "yaw=-90:pitch=0"),
            (3.0, "yaw=90:pitch=0"),
            (4.0, "yaw=180:pitch=0"),
            (5.0, "yaw=0:pitch=45"),
            (6.0, "yaw=0:pitch=-45"),
        ];

        for (preset, rotation) in cases {
            assert_eq!(
                state.dispatch(Action::control("panorama-360", preset).unwrap()),
                vec![Effect::Engine(vec![
                    "vf".into(),
                    "add".into(),
                    format!(
                        "@foxide-panorama360:lavfi=[v360=input=e:output=flat:w=1280:h=720:h_fov=90:v_fov=60:{rotation}]"
                    ),
                ])]
            );
        }

        assert_eq!(
            state.dispatch(Action::control("panorama-360", 0.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf".into(),
                "remove".into(),
                "@foxide-panorama360".into(),
            ])]
        );
    }

    #[test]
    fn panorama_360_rejects_non_integer_or_out_of_range_presets() {
        for preset in [-1.0, 0.5, 1.5, 7.0, f64::NAN, f64::INFINITY] {
            assert!(Action::control("panorama-360", preset).is_none());
        }
    }

    #[test]
    fn panorama_360_drag_updates_the_active_projection_without_replacing_it() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);
        state.dispatch(Action::control("panorama-360", 1.0).unwrap());

        assert_eq!(
            state.dispatch(Action::control("panorama-yaw", -12.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf-command".into(),
                "@foxide-panorama360".into(),
                "yaw".into(),
                "-12".into(),
                "v360".into(),
            ])]
        );
        assert_eq!(
            state.dispatch(Action::control("panorama-yaw", 180.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf-command".into(),
                "@foxide-panorama360".into(),
                "yaw".into(),
                "168".into(),
                "v360".into(),
            ])]
        );
        assert_eq!(
            state.dispatch(Action::control("panorama-pitch", 120.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf-command".into(),
                "@foxide-panorama360".into(),
                "pitch".into(),
                "89".into(),
                "v360".into(),
            ])]
        );
        assert!(Action::control("panorama-yaw", 181.0).is_none());
        state.dispatch(Action::control("panorama-360", 0.0).unwrap());
        assert!(
            state
                .dispatch(Action::control("panorama-yaw", 1.0).unwrap())
                .is_empty()
        );
    }

    #[test]
    fn stereo_3d_control_covers_supported_source_layouts_and_can_remove_only_its_filter() {
        let mut state = UiState::default();
        state.confirm_loaded(42.0);
        let cases = [
            (2.0, "in=sbsl:out=abl"),
            (3.0, "in=sbsr:out=arcd"),
            (4.0, "in=sbsr:out=sbsl"),
            (5.0, "in=abl:out=arcd"),
            (6.0, "in=abl:out=sbsl"),
        ];

        for (mode, filter) in cases {
            assert_eq!(
                state.dispatch(Action::control("stereo-3d", mode).unwrap()),
                vec![Effect::Engine(vec![
                    "vf".into(),
                    "add".into(),
                    format!("@foxide-stereo3d:lavfi=[stereo3d={filter}]"),
                ])]
            );
        }

        assert_eq!(
            state.dispatch(Action::control("stereo-3d", 0.0).unwrap()),
            vec![Effect::Engine(vec![
                "vf".into(),
                "remove".into(),
                "@foxide-stereo3d".into(),
            ])]
        );
    }

    #[test]
    fn stereo_3d_rejects_non_integer_or_out_of_range_modes() {
        for mode in [-1.0, 0.5, 1.5, 7.0, f64::NAN, f64::INFINITY] {
            assert!(Action::control("stereo-3d", mode).is_none());
        }
    }

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
