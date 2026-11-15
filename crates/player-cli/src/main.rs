use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use player_core::PlaybackOptions;
use std::{ffi::OsString, path::PathBuf, process::Command};

/// Foxide development launcher (external mpv; custom Rust window is a later milestone).
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Files, folders, playlists, or HTTP(S)/RTSP stream URLs.
    #[arg(required = true)]
    media: Vec<OsString>,
    /// Override the automatically discovered local mpv executable.
    #[arg(long)]
    mpv: Option<PathBuf>,
    #[arg(long, default_value_t = 1.0)]
    speed: f64,
    #[arg(long)]
    audio_track: Option<u32>,
    #[arg(long)]
    subtitle_track: Option<u32>,
    #[arg(long = "subtitle")]
    subtitle_files: Vec<PathBuf>,
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    audio_delay: f64,
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    subtitle_delay: f64,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    brightness: i32,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    contrast: i32,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    gamma: i32,
    #[arg(long)]
    deinterlace: bool,
    /// Explicit HDR-to-SDR output (BT.709 / BT.1886).
    #[arg(long)]
    tone_map: bool,
    #[arg(long, conflicts_with = "chapter")]
    start: Option<f64>,
    /// Zero-based chapter index.
    #[arg(long)]
    chapter: Option<u32>,
    /// Save/restore mpv watch-later state in this directory.
    #[arg(long)]
    resume_dir: Option<PathBuf>,
    #[arg(long)]
    minimal: bool,
    #[arg(long)]
    on_top: bool,
    #[arg(long)]
    loop_playlist: bool,
    #[arg(long, requires = "ab_end")]
    ab_start: Option<f64>,
    #[arg(long, requires = "ab_start")]
    ab_end: Option<f64>,
    /// Queue this file's folder in natural episode order, starting with this file.
    #[arg(long)]
    autoqueue: bool,
    /// Print the executable and arguments as JSON without starting playback.
    #[arg(long)]
    print_command: bool,
    /// Diagnostic software decoding with no audio/video output.
    #[arg(long)]
    headless: bool,
    #[arg(long)]
    paused: bool,
    /// Explicit event collector script for benchmark runs.
    #[arg(long)]
    benchmark_script: Option<PathBuf>,
}

fn mpv_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit.or_else(|| std::env::var_os("FOXIDE_MPV").map(PathBuf::from)) {
        return path;
    }
    let filename = if cfg!(windows) { "mpv.exe" } else { "mpv" };
    let mut roots = Vec::with_capacity(3);
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.ancestors().skip(1).take(3).map(PathBuf::from));
    }
    let mut seen = std::collections::HashSet::new();
    let suffix = PathBuf::from("tools/mpv").join(filename);
    let bundled = roots
        .into_iter()
        .filter(|root| seen.insert(root.clone()))
        .map(|root| root.join(&suffix))
        .find(|path| path.is_file());
    if let Some(path) = bundled {
        return path;
    }
    // Resolve PATH explicitly: a bare Windows command also searches the working
    // directory, even when that directory is absent from PATH.
    if let Some(path) = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(filename))
        .find(|path| path.is_file())
    {
        return path;
    }
    // Keep failure anchored to the application, never an implicit cwd search.
    std::env::current_exe()
        .expect("Cannot locate Foxide executable")
        .with_file_name(filename)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = PlaybackOptions {
        speed: args.speed,
        audio_track: args.audio_track,
        subtitle_track: args.subtitle_track,
        subtitle_files: args
            .subtitle_files
            .into_iter()
            .map(|p| p.canonicalize().context("Subtitle file does not exist"))
            .collect::<Result<Vec<_>>>()?,
        audio_delay: args.audio_delay,
        subtitle_delay: args.subtitle_delay,
        brightness: args.brightness,
        contrast: args.contrast,
        gamma: args.gamma,
        deinterlace: args.deinterlace,
        tone_map: args.tone_map,
        start: args.start,
        chapter: args.chapter,
        resume_dir: args.resume_dir,
        minimal: args.minimal,
        on_top: args.on_top,
        loop_playlist: args.loop_playlist,
        ab_loop: args.ab_start.zip(args.ab_end),
        paused: args.paused,
    };
    let mut engine_args = options.mpv_args().map_err(anyhow::Error::msg)?;
    if let Some(script) = args.benchmark_script {
        let mut argument = OsString::from("--script=");
        argument.push(player_core::external_path(
            script
                .canonicalize()
                .context("Benchmark script does not exist")?,
        ));
        engine_args.push(argument);
    }
    let mut media = Vec::with_capacity(args.media.len());
    ensure!(
        !args.autoqueue || args.media.len() == 1,
        "autoqueue takes one starting file"
    );
    for input in args.media {
        if input.to_str().is_some_and(|s| {
            ["http://", "https://", "rtsp://"]
                .iter()
                .any(|scheme| s.starts_with(scheme))
        }) {
            ensure!(!args.autoqueue, "folder autoqueue requires a local file");
            media.push(input);
            continue;
        }
        let path = PathBuf::from(input)
            .canonicalize()
            .context("Media file does not exist")?;
        let is_directory = path.is_dir();
        if is_directory || args.autoqueue {
            let folder = if is_directory {
                path.as_path()
            } else {
                path.parent().context("File has no parent folder")?
            };
            let queue = player_core::folder_queue(folder).context("Reading folder queue")?;
            ensure!(
                !queue.is_empty(),
                "Folder contains no recognized media files"
            );
            let start = if args.autoqueue && path.is_file() {
                queue
                    .iter()
                    .position(|p| p == &path)
                    .context("Starting file is not recognized by the folder queue")?
            } else {
                0
            };
            engine_args.push(format!("--playlist-start={start}").into());
            media.reserve(queue.len());
            media.extend(
                queue
                    .into_iter()
                    .map(player_core::external_path)
                    .map(PathBuf::into_os_string),
            );
        } else {
            media.push(player_core::external_path(path).into_os_string());
        }
    }
    if args.headless {
        engine_args.extend(["--vo=null".into(), "--ao=null".into(), "--hwdec=no".into()]);
    }
    engine_args.reserve(1 + media.len());
    engine_args.push("--".into());
    engine_args.extend(media);
    let executable = mpv_path(args.mpv);
    if args.print_command {
        let command: Vec<_> = std::iter::once(executable.as_os_str())
            .chain(engine_args.iter().map(OsString::as_os_str))
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        println!("{}", serde_json::to_string_pretty(&command)?);
        return Ok(());
    }
    eprintln!("Foxide bootstrap: playback is provided by an external mpv process.");
    let status = Command::new(executable)
        .args(engine_args)
        .status()
        .context("Install mpv or supply --mpv <path-to-mpv>")?;
    if !status.success() {
        bail!("mpv exited with {status}");
    }
    Ok(())
}
