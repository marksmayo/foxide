use std::{ffi::OsString, path::PathBuf};

/// Validated playback controls supplied to the current external-mpv adapter.
#[derive(Debug, Clone)]
pub struct PlaybackOptions {
    pub speed: f64,
    pub audio_track: Option<u32>,
    pub subtitle_track: Option<u32>,
    pub subtitle_files: Vec<PathBuf>,
    pub audio_delay: f64,
    pub subtitle_delay: f64,
    pub brightness: i32,
    pub contrast: i32,
    pub gamma: i32,
    pub deinterlace: bool,
    pub tone_map: bool,
    pub start: Option<f64>,
    pub chapter: Option<u32>,
    pub resume_dir: Option<PathBuf>,
    pub minimal: bool,
    pub on_top: bool,
    pub loop_playlist: bool,
    pub ab_loop: Option<(f64, f64)>,
    pub paused: bool,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            speed: 1.0,
            audio_track: None,
            subtitle_track: None,
            subtitle_files: vec![],
            audio_delay: 0.0,
            subtitle_delay: 0.0,
            brightness: 0,
            contrast: 0,
            gamma: 0,
            deinterlace: false,
            tone_map: false,
            start: None,
            chapter: None,
            resume_dir: None,
            minimal: false,
            on_top: false,
            loop_playlist: false,
            ab_loop: None,
            paused: false,
        }
    }
}

impl PlaybackOptions {
    pub fn mpv_args(&self) -> Result<Vec<OsString>, String> {
        if !self.speed.is_finite() || !(0.25..=4.0).contains(&self.speed) {
            return Err("speed must be between 0.25 and 4.0".into());
        }
        if !self.audio_delay.is_finite() || !self.subtitle_delay.is_finite() {
            return Err("audio and subtitle delays must be finite".into());
        }
        if self.audio_track == Some(0) || self.subtitle_track == Some(0) {
            return Err("track IDs start at 1".into());
        }
        if [self.brightness, self.contrast, self.gamma]
            .iter()
            .any(|v| !(-100..=100).contains(v))
        {
            return Err("picture adjustments must be between -100 and 100".into());
        }
        if self.start.is_some_and(|v| !v.is_finite() || v < 0.0) {
            return Err("start time must be finite and nonnegative".into());
        }
        if self.start.is_some() && self.chapter.is_some() {
            return Err("choose a start time or a chapter, not both".into());
        }
        if self
            .ab_loop
            .is_some_and(|(a, b)| !a.is_finite() || !b.is_finite() || a < 0.0 || b <= a)
        {
            return Err("A–B loop requires finite 0 <= A < B".into());
        }
        let mut args = vec![
            "--no-config".into(),
            "--load-scripts=no".into(),
            "--title=Foxide".into(),
            format!("--hwdec={}", crate::HWDEC).into(),
            format!("--speed={}", self.speed).into(),
            format!("--audio-delay={}", self.audio_delay).into(),
            format!("--sub-delay={}", self.subtitle_delay).into(),
        ];
        if let Some(track) = self.audio_track {
            args.push(format!("--aid={track}").into());
        }
        if let Some(track) = self.subtitle_track {
            args.push(format!("--sid={track}").into());
        }
        for file in &self.subtitle_files {
            let mut argument = OsString::from("--sub-file=");
            argument.push(crate::external_path(file.clone()));
            args.push(argument);
        }
        for (name, value) in [
            ("brightness", self.brightness),
            ("contrast", self.contrast),
            ("gamma", self.gamma),
        ] {
            args.push(format!("--{name}={value}").into());
        }
        if self.deinterlace {
            args.push("--deinterlace=yes".into());
        }
        if self.tone_map {
            args.extend([
                "--vo=gpu-next".into(),
                "--tone-mapping=bt.2390".into(),
                "--target-trc=bt.1886".into(),
                "--target-prim=bt.709".into(),
            ]);
        }
        if let Some(start) = self.start {
            args.push(format!("--start={start}").into());
        }
        if let Some(chapter) = self.chapter {
            args.push(format!("--start=#{chapter}").into());
        }
        if let Some(directory) = &self.resume_dir {
            args.extend([
                "--save-position-on-quit=yes".into(),
                "--resume-playback=yes".into(),
                "--watch-later-options=start".into(),
            ]);
            let mut argument = OsString::from("--watch-later-directory=");
            argument.push(crate::external_path(directory.clone()));
            args.push(argument);
        }
        if self.minimal {
            args.push("--border=no".into());
        }
        if self.on_top {
            args.push("--ontop=yes".into());
        }
        if self.loop_playlist {
            args.push("--loop-playlist=inf".into());
        }
        if self.paused {
            args.push("--pause=yes".into());
        }
        if let Some((a, b)) = self.ab_loop {
            args.extend([
                format!("--ab-loop-a={a}").into(),
                format!("--ab-loop-b={b}").into(),
            ]);
        }
        Ok(args)
    }
}
