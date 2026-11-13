mod model;
mod report;
mod runner;

use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(
    version,
    about = "Foxide process benchmark harness; display and power metrics require external collectors"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List environment and tools without launching players.
    Doctor,
    /// Launch and sample mpv, VLC, or a directly launched external executable.
    Run(RunArgs),
    /// Sample an existing process tree (native apps); startup timing is unavailable.
    Observe(ObserveArgs),
    /// Render JSON runs into a Markdown comparison table.
    Report {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum Player {
    Mpv,
    Vlc,
    External,
}

#[derive(Clone, Debug, PartialEq, ValueEnum)]
enum Cache {
    Warm,
    Uncontrolled,
    ColdReboot,
}

#[derive(Args)]
struct Sampling {
    /// Total sample period, including warmup. Use 630 for 30s warmup + 10min playback.
    #[arg(long, default_value_t = 630.0)]
    duration: f64,
    #[arg(long, default_value_t = 30.0)]
    warmup: f64,
    #[arg(long, default_value_t = 250)]
    sample_ms: u64,
    #[arg(long, default_value = "results/local")]
    output: PathBuf,
}

#[derive(Args)]
struct RunArgs {
    #[arg(long, value_enum)]
    player: Player,
    #[arg(long)]
    exe: Option<PathBuf>,
    #[arg(long)]
    media: PathBuf,
    #[arg(long, default_value_t = 5)]
    repeat: u32,
    #[arg(long, value_enum, default_value = "uncontrolled")]
    cache: Cache,
    #[arg(long)]
    cache_evidence: Option<String>,
    /// mpv only: explicitly request auto-safe hardware decoding. Default is clean player defaults.
    #[arg(long)]
    tuned: bool,
    /// mpv only: separate scripted +30s keyframe and ten exact-seek proxy test.
    #[arg(long)]
    seeks: bool,
    /// Additional player arguments, each passed as one argument (use --arg=--option=value).
    #[arg(long = "arg", allow_hyphen_values = true)]
    extra: Vec<String>,
    #[command(flatten)]
    sampling: Sampling,
}

#[derive(Args)]
struct ObserveArgs {
    #[arg(long)]
    pid: u32,
    #[arg(long)]
    label: String,
    #[arg(long)]
    media: PathBuf,
    #[command(flatten)]
    sampling: Sampling,
}

fn validate(s: &Sampling) -> Result<()> {
    ensure!(s.duration <= 86400.0, "duration must not exceed one day");
    ensure!(
        s.duration.is_finite() && s.warmup.is_finite(),
        "Durations must be finite"
    );
    ensure!(
        s.warmup >= 0.0 && s.duration > s.warmup,
        "duration must exceed nonnegative warmup"
    );
    ensure!(
        s.sample_ms >= 100,
        "sample-ms must be >= 100 to bound observer overhead"
    );
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Doctor => runner::doctor(),
        Commands::Run(args) => {
            validate(&args.sampling)?;
            ensure!(args.repeat > 0, "repeat must be positive");
            ensure!(
                !args.tuned || matches!(args.player, Player::Mpv),
                "tuned requires mpv"
            );
            ensure!(
                !args.seeks || matches!(args.player, Player::Mpv),
                "seeks requires mpv"
            );
            ensure!(
                args.cache != Cache::ColdReboot
                    || (args.repeat == 1
                        && args
                            .cache_evidence
                            .as_ref()
                            .is_some_and(|e| !e.trim().is_empty())),
                "cold-reboot requires repeat=1 and --cache-evidence describing reboot and cache control"
            );
            runner::run(args)
        }
        Commands::Observe(args) => {
            validate(&args.sampling)?;
            runner::observe(args)
        }
        Commands::Report { input, output } => {
            let markdown = report::render(&input)?;
            if let Some(output) = output {
                fs::write(output, markdown).context("Writing report")?;
            } else {
                print!("{markdown}");
            }
            Ok(())
        }
    }
}
