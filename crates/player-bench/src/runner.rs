use crate::{
    Cache, ObserveArgs, Player, RunArgs, Sampling,
    model::{Run, Sample, summarize},
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TELEMETRY: &str = include_str!("../../../scripts/telemetry.py");
const LUA: &str = include_str!("../assets/bench.lua");

struct ManagedChild(Child);
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn directory(output: &Path, label: &str) -> Result<PathBuf> {
    fs::create_dir_all(output)?;
    let dir = output.join(format!("{}-{}-{label}", unix_ms(), std::process::id()));
    fs::create_dir(&dir).context("Creating unique run directory")?;
    dir.canonicalize()
        .map(player_core::external_path)
        .context("Resolving output directory")
}

fn python() -> PathBuf {
    std::env::var_os("FOXIDE_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| "python".into())
}

fn sampler(s: &Sampling, dir: &Path) -> Result<ManagedChild> {
    let script = dir.join("telemetry.py");
    fs::write(&script, TELEMETRY)?;
    let mut child = ManagedChild(
        Command::new(python())
            .arg(script)
            .arg("--output")
            .arg(dir.join("samples.jsonl"))
            .args([
                "--duration",
                &s.duration.to_string(),
                "--interval-ms",
                &s.sample_ms.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(File::create(dir.join("telemetry.stderr.log"))?)
            .spawn()
            .context("Starting sampler; install Python 3.10+ or set FOXIDE_PYTHON")?,
    );
    let stdout = child
        .0
        .stdout
        .take()
        .context("Sampler stdout unavailable")?;
    let mut ready = String::new();
    BufReader::new(stdout).read_line(&mut ready)?;
    ensure!(
        ready.trim() == "ready",
        "Sampler failed to initialize; see telemetry.stderr.log"
    );
    Ok(child)
}

fn base_run(player: String, media: &Path, s: &Sampling) -> Run {
    Run {
        schema_version: 1, player, profile: "clean-defaults".into(),
        media: media.to_string_lossy().into_owned(), executable: String::new(), args: vec![],
        os: std::env::consts::OS.into(), arch: std::env::consts::ARCH.into(),
        host: std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "unknown".into()),
        cache: "uncontrolled".into(), cache_evidence: None, pid: 0,
        started_unix_ms: 0, duration_seconds: s.duration, warmup_seconds: s.warmup,
        sample_interval_ms: s.sample_ms, exit_status: None, terminated_by_harness: false,
        status: "process-sampled-playback-unverified".into(),
        notes: vec!["RSS is sampled, not an allocation high-water mark; tree sum may double-count shared pages.".into(),
            "CPU uses 100% per logical core; first sample has no CPU delta. GPU, power, display TTFF and pacing are unmeasured.".into(),
            "Run includes harness instrumentation; quantify observer overhead before using small differences.".into()],
        samples: vec![], rss_peak_bytes: None, rss_steady_mean_bytes: None,
        cpu_steady_mean_percent_one_core: None, startup_restart_observed_ms: None,
        ttff_display_ms: None, package_power_watts: None, gpu_utilization_percent: None,
        vram_bytes: None, frame_pacing_stddev_ms: None, instrumentation: vec![],
    }
}

fn finish_samples(
    run: &mut Run,
    sampler: &mut ManagedChild,
    dir: &Path,
    offset_ms: f64,
) -> Result<()> {
    let timeout = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = sampler.0.try_wait()? {
            break status;
        }
        ensure!(Instant::now() < timeout, "Sampler did not exit on schedule");
        thread::sleep(Duration::from_millis(20));
    };
    ensure!(status.success(), "Sampler failed; see telemetry.stderr.log");
    for line in fs::read_to_string(dir.join("samples.jsonl"))?.lines() {
        let record: serde_json::Value =
            serde_json::from_str(line).context("Invalid telemetry JSON")?;
        if record["kind"] == "sample" {
            let mut sample: Sample = serde_json::from_value(record.clone())?;
            sample.elapsed_ms += offset_ms;
            run.samples.push(sample);
        } else if record["kind"] == "pid-reused" || record["kind"] == "root-exited-or-inaccessible"
        {
            run.status = "incomplete-process-exited-or-inaccessible".into();
            run.instrumentation.push(record);
        } else {
            run.instrumentation.push(record);
        }
    }
    if run.samples.is_empty() {
        run.status = "no-accessible-samples".into();
    }
    summarize(run);
    Ok(())
}

fn save(run: &Run, dir: &Path, prime: bool) -> Result<()> {
    let path = dir.join(if prime { "priming.json" } else { "run.json" });
    fs::write(&path, serde_json::to_vec_pretty(run)?)?;
    println!("{} [{}]", path.display(), run.status);
    Ok(())
}

fn events(run: &mut Run, dir: &Path) -> Result<()> {
    let path = dir.join("events.jsonl");
    if !path.exists() {
        return Ok(());
    }
    for line in fs::read_to_string(path)?.lines() {
        let event: serde_json::Value =
            serde_json::from_str(line).context("Invalid mpv event JSON")?;
        if event["kind"] == "end-file" {
            run.status = "incomplete-end-file".into();
        }
        run.instrumentation.push(event);
    }
    Ok(())
}

fn one(args: &RunArgs, prime: bool, index: u32) -> Result<bool> {
    let media = args
        .media
        .canonicalize()
        .context("Media file does not exist")?;
    let media = player_core::external_path(media);
    ensure!(media.is_file(), "Media must be a local file");
    let dir = directory(
        &args.sampling.output,
        &format!("{}-{index}", if prime { "prime" } else { "run" }),
    )?;
    let mut settings = Sampling {
        duration: args.sampling.duration,
        warmup: args.sampling.warmup,
        sample_ms: args.sampling.sample_ms,
        output: args.sampling.output.clone(),
    };
    if prime {
        settings.duration = 3.0;
        settings.warmup = 0.0;
    }
    let mut telemetry = sampler(&settings, &dir)?;
    let name = match args.player {
        Player::Mpv => "mpv",
        Player::Vlc => "vlc",
        Player::External => "external",
    };
    let exe = args.exe.clone().unwrap_or_else(|| name.into());
    ensure!(
        !matches!(args.player, Player::External) || args.exe.is_some(),
        "external requires --exe with the actual player binary (not a launcher)"
    );
    let mut options: Vec<String> = match args.player {
        Player::Mpv => vec![
            "--no-config".into(),
            "--load-scripts=no".into(),
            "--no-terminal".into(),
            "--force-window=yes".into(),
            "--keep-open=yes".into(),
            format!("--script={}", dir.join("bench.lua").display()),
        ],
        Player::Vlc => {
            let mut options = vec![
                "--ignore-config".into(),
                "--no-video-title-show".into(),
                "--play-and-exit".into(),
            ];
            // The native macOS VLC build does not expose this Qt/Windows option.
            if !cfg!(target_os = "macos") {
                options.push("--no-one-instance".into());
            }
            options
        }
        Player::External => vec![],
    };
    if args.tuned {
        options.push(format!("--hwdec={}", player_core::HWDEC));
    }
    options.extend(args.extra.clone());
    options.push("--".into());
    options.push(media.to_string_lossy().into_owned());
    // External native binaries may not accept '--'; use observe for native app launchers.
    if matches!(args.player, Player::External) {
        options.remove(options.len() - 2);
    }
    fs::write(dir.join("bench.lua"), LUA)?;
    let mut run = base_run(name.into(), &media, &settings);
    run.executable = exe.to_string_lossy().into_owned();
    run.profile = if args.tuned {
        "hwdec-auto-safe"
    } else {
        "clean-defaults"
    }
    .into();
    if args.seeks && !prime {
        run.profile.push_str("-seeks");
    }
    run.args = options.clone();
    run.cache = match args.cache {
        Cache::Warm => "warm-primed",
        Cache::Uncontrolled => "uncontrolled",
        Cache::ColdReboot => "cold-reboot-user-attested",
    }
    .into();
    run.cache_evidence = args.cache_evidence.clone();
    let mut command = Command::new(&exe);
    command
        .args(&options)
        .stdin(Stdio::null())
        .stdout(File::create(dir.join("player.stdout.log"))?)
        .stderr(File::create(dir.join("player.stderr.log"))?)
        .env("FOXIDE_BENCH_EVENTS", dir.join("events.jsonl"))
        .env(
            "FOXIDE_BENCH_SEEKS",
            if args.seeks && !prime { "1" } else { "0" },
        );
    run.started_unix_ms = unix_ms();
    let started = Instant::now();
    let mut player = ManagedChild(
        command
            .spawn()
            .with_context(|| format!("Launching {}", exe.display()))?,
    );
    run.pid = player.0.id();
    let sample_offset_ms = started.elapsed().as_secs_f64() * 1000.0;
    writeln!(
        telemetry
            .0
            .stdin
            .take()
            .context("Sampler stdin unavailable")?,
        "{}",
        run.pid
    )?;
    println!(
        "Sampling {name} PID {} for {}s{}",
        run.pid,
        settings.duration,
        if prime { " (warm-cache priming)" } else { "" }
    );
    let mut event_reader: Option<BufReader<File>> = None;
    let mut pending_line = String::new();
    let deadline = started + Duration::from_secs_f64(settings.duration);
    while Instant::now() < deadline {
        if event_reader.is_none() && dir.join("events.jsonl").exists() {
            event_reader = Some(BufReader::new(File::open(dir.join("events.jsonl"))?));
        }
        if run.startup_restart_observed_ms.is_none()
            && let Some(reader) = event_reader.as_mut()
        {
            loop {
                let count = reader.read_line(&mut pending_line)?;
                if count == 0 {
                    break;
                }
                if !pending_line.ends_with('\n') {
                    break;
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&pending_line)
                    && value["kind"] == "first-playback-restart-proxy"
                {
                    run.startup_restart_observed_ms =
                        Some(started.elapsed().as_secs_f64() * 1000.0);
                    run.status = "internal-restart-observed".into();
                }
                pending_line.clear();
            }
        }
        if let Some(status) = player.0.try_wait()? {
            run.exit_status = Some(status.to_string());
            run.status = "incomplete-player-exited".into();
            break;
        }
        thread::sleep(Duration::from_millis(
            if run.startup_restart_observed_ms.is_none() {
                5
            } else {
                25
            },
        ));
    }
    // Wait for the last sample BEFORE terminating the player, avoiding false 'root exited'.
    let samples_result = finish_samples(&mut run, &mut telemetry, &dir, sample_offset_ms);
    if player.0.try_wait()?.is_none() {
        player.0.kill()?;
        run.exit_status = Some(player.0.wait()?.to_string());
        run.terminated_by_harness = true;
    }
    if let Err(error) = samples_result {
        run.status = "telemetry-failed".into();
        run.notes.push(format!("{error:#}"));
    }
    events(&mut run, &dir)?;
    if args.seeks
        && !prime
        && !run
            .instrumentation
            .iter()
            .any(|e| e["kind"] == "seeks-complete")
    {
        run.status = "incomplete-seeks".into();
    }
    if matches!(args.player, Player::Mpv) && run.startup_restart_observed_ms.is_none() {
        run.status = "incomplete-no-playback-restart".into();
    }
    let valid = matches!(
        run.status.as_str(),
        "process-sampled-playback-unverified" | "internal-restart-observed"
    );
    save(&run, &dir, prime)?;
    Ok(valid)
}

pub fn run(args: RunArgs) -> Result<()> {
    if args.cache == Cache::Warm {
        ensure!(
            one(&args, true, 0)?,
            "Warm-cache priming failed; inspect the saved logs"
        );
    }
    let mut all_valid = true;
    for index in 1..=args.repeat {
        all_valid &= one(&args, false, index)?;
    }
    ensure!(
        all_valid,
        "One or more runs are incomplete; raw results and logs were preserved"
    );
    Ok(())
}

pub fn observe(args: ObserveArgs) -> Result<()> {
    let media = args.media.canonicalize().context("Media must exist")?;
    let dir = directory(&args.sampling.output, "observe")?;
    let mut telemetry = sampler(&args.sampling, &dir)?;
    let mut run = base_run(args.label, &media, &args.sampling);
    run.profile = "attach-existing-startup-unavailable".into();
    run.pid = args.pid;
    run.started_unix_ms = unix_ms();
    run.notes.push("Process was already running; verify its executable, media and playback separately. Observe never terminates the attached player.".into());
    writeln!(
        telemetry
            .0
            .stdin
            .take()
            .context("Sampler stdin unavailable")?,
        "{}",
        args.pid
    )?;
    let deadline = Instant::now() + Duration::from_secs_f64(args.sampling.duration + 5.0);
    while telemetry.0.try_wait()?.is_none() {
        ensure!(Instant::now() < deadline, "Sampler exceeded duration");
        thread::sleep(Duration::from_millis(100));
    }
    finish_samples(&mut run, &mut telemetry, &dir, 0.0)?;
    let valid = !run.samples.is_empty() && run.status == "process-sampled-playback-unverified";
    save(&run, &dir, false)?;
    ensure!(
        valid,
        "No complete process observation; inspect saved results"
    );
    Ok(())
}

fn locate(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for suffix in if cfg!(windows) {
            &[".exe", ""][..]
        } else {
            &[""][..]
        } {
            let candidate = dir.join(format!("{name}{suffix}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn doctor() -> Result<()> {
    println!(
        "Foxide player-bench {} | {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for tool in [
        "python",
        "ffmpeg",
        "ffprobe",
        "mpv",
        "vlc",
        "wpr",
        "typeperf",
        "PresentMon",
        "powermetrics",
        "perf",
        "powertop",
    ] {
        println!(
            "{tool:14} {}",
            locate(tool).map_or_else(|| "not on PATH".into(), |p| p.display().to_string())
        );
    }
    if cfg!(windows) {
        let vlc = Path::new("C:/Program Files/VideoLAN/VLC/vlc.exe");
        if vlc.exists() {
            println!("VLC fallback   {} (pass --exe)", vlc.display());
        }
    }
    println!(
        "Native app activation may reuse a process. Use observe --pid with the verified media process."
    );
    println!(
        "Display TTFF, VRAM and package watts require external telemetry; see docs/benchmarking.md."
    );
    Ok(())
}
