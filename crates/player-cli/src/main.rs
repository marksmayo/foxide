use anyhow::{Context, Result, bail};
use clap::Parser;
use std::{path::PathBuf, process::Command};

/// Foxide development launcher (external mpv; custom Rust window is a later milestone).
#[derive(Parser)]
#[command(version, about)]
struct Args {
    media: PathBuf,
    /// Override the automatically discovered local mpv executable.
    #[arg(long)]
    mpv: Option<PathBuf>,
}

fn mpv_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit.or_else(|| std::env::var_os("FOXIDE_MPV").map(PathBuf::from)) {
        return path;
    }
    let filename = if cfg!(windows) { "mpv.exe" } else { "mpv" };
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.ancestors().skip(1).take(3).map(PathBuf::from));
    }
    roots.into_iter().map(|root| root.join("tools/mpv").join(filename))
        .find(|path| path.is_file()).unwrap_or_else(|| "mpv".into())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let media = args
        .media
        .canonicalize()
        .context("Media file does not exist")?;
    let media = player_core::external_path(media);
    eprintln!("Foxide bootstrap: playback is provided by an external mpv process.");
    let status = Command::new(mpv_path(args.mpv))
        .args(["--no-config", "--load-scripts=no", "--title=Foxide"])
        .arg(format!("--hwdec={}", player_core::HWDEC))
        .arg("--")
        .arg(media)
        .status()
        .context("Install mpv or supply --mpv <path-to-mpv>")?;
    if !status.success() {
        bail!("mpv exited with {status}");
    }
    Ok(())
}
