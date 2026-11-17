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
        started_unix_ms: 0, sampling_started_ms: 0.0, duration_seconds: s.duration, warmup_seconds: s.warmup,
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
    for line in BufReader::new(File::open(dir.join("samples.jsonl"))?).lines() {
        let line = line?;
        let record: serde_json::Value =
            serde_json::from_str(&line).context("Invalid telemetry JSON")?;
        if record["kind"] == "sample" {
            let mut sample: Sample = serde_json::from_value(record)?;
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
    let mut output = std::io::BufWriter::new(File::create(&path)?);
    serde_json::to_writer_pretty(&mut output, run)?;
    output.flush()?;
    println!("{} [{}]", path.display(), run.status);
    Ok(())
}

fn events(run: &mut Run, dir: &Path) -> Result<()> {
    let path = dir.join("events.jsonl");
    if !path.exists() {
        return Ok(());
    }
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        let event: serde_json::Value =
            serde_json::from_str(&line).context("Invalid mpv event JSON")?;
        if event["kind"] == "end-file" {
            run.status = "incomplete-end-file".into();
        }
        run.instrumentation.push(event);
    }
    Ok(())
}

fn finish_player(run: &mut Run, player: &mut Child) -> Result<()> {
    match player.try_wait()? {
        Some(status) => {
            run.exit_status = Some(status.to_string());
            run.status = "incomplete-player-exited".into();
        }
        None => {
            player.kill()?;
            run.exit_status = Some(player.wait()?.to_string());
            run.terminated_by_harness = true;
        }
    }
    Ok(())
}

/// Internal readiness only: never a claim of display presentation.
fn wait_for_startup(
    run: &mut Run,
    player: &mut Child,
    event_path: &Path,
    started: Instant,
    timeout: f64,
) -> Result<bool> {
    let deadline = started + Duration::from_secs_f64(timeout);
    let mut reader = None;
    let mut pending = String::new();
    loop {
        if reader.is_none() && event_path.exists() {
            reader = Some(BufReader::new(File::open(event_path)?));
        }
        if let Some(reader) = reader.as_mut() {
            loop {
                let count = reader.read_line(&mut pending)?;
                if count == 0 || !pending.ends_with('\n') {
                    break;
                }
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&pending)
                    && event["kind"] == "first-playback-restart-proxy"
                {
                    run.startup_restart_observed_ms =
                        Some(started.elapsed().as_secs_f64() * 1000.0);
                    run.status = "internal-restart-observed".into();
                    return Ok(true);
                }
                pending.clear();
            }
        }
        if let Some(status) = player.try_wait()? {
            run.exit_status = Some(status.to_string());
            run.status = "incomplete-player-exited".into();
            return Ok(false);
        }
        if Instant::now() >= deadline {
            run.status = "incomplete-startup-timeout".into();
            return Ok(false);
        }
        thread::sleep(Duration::from_millis(5));
    }
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
    if matches!(args.player, Player::Mpv) {
        run.profile.push_str("-post-restart-sampling");
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
    if matches!(args.player, Player::Mpv)
        && !wait_for_startup(
            &mut run,
            &mut player.0,
            &dir.join("events.jsonl"),
            started,
            args.startup_timeout,
        )?
    {
        finish_player(&mut run, &mut player.0)?;
        save(&run, &dir, prime)?;
        return Ok(false);
    }
    let measurement_started = Instant::now();
    run.sampling_started_ms = started.elapsed().as_secs_f64() * 1000.0;
    let sample_offset_ms = run.sampling_started_ms;
    run.notes.push("Sampling duration and warmup begin at sampling_started_ms; sample elapsed times remain relative to process spawn. mpv readiness is an internal restart proxy, not displayed TTFF.".into());
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
    let deadline = measurement_started + Duration::from_secs_f64(settings.duration);
    while Instant::now() < deadline {
        if let Some(status) = player.0.try_wait()? {
            run.exit_status = Some(status.to_string());
            run.status = "incomplete-player-exited".into();
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    // Wait for the last sample BEFORE terminating the player, avoiding false 'root exited'.
    let samples_result = finish_samples(&mut run, &mut telemetry, &dir, sample_offset_ms);
    finish_player(&mut run, &mut player.0)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_run() -> Run {
        base_run(
            "external".into(),
            Path::new("test-fixture"),
            &Sampling {
                duration: 1.0,
                warmup: 0.0,
                sample_ms: 100,
                output: PathBuf::new(),
            },
        )
    }

    #[test]
    fn summaries_reset_missing_values_and_respect_the_sampling_origin() {
        let mut run = fixture_run();
        run.sampling_started_ms = 4500.0;
        run.warmup_seconds = 0.1;
        run.samples = vec![
            Sample {
                elapsed_ms: 4550.0,
                rss_bytes: 100,
                cpu_percent_one_core: Some(10.0),
                process_count: 1,
            },
            Sample {
                elapsed_ms: 4650.0,
                rss_bytes: 200,
                cpu_percent_one_core: Some(20.0),
                process_count: 1,
            },
        ];
        summarize(&mut run);
        assert_eq!(run.rss_steady_mean_bytes, Some(200.0));
        assert_eq!(run.cpu_steady_mean_percent_one_core, Some(20.0));
        run.samples.clear();
        summarize(&mut run);
        assert_eq!(run.rss_peak_bytes, None);
        assert_eq!(run.rss_steady_mean_bytes, None);
        assert_eq!(run.cpu_steady_mean_percent_one_core, None);
    }

    #[test]
    fn exit_during_final_sample_is_recorded_as_incomplete() {
        for code in [0, 7] {
            let mut child = ManagedChild(
                Command::new(python())
                    .args(["-c", &format!("raise SystemExit({code})")])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            );
            // Reproduce the exact cleanup state without a timing race: the child
            // has exited after the polling loop and before final classification.
            assert_eq!(child.0.wait().unwrap().code(), Some(code));
            let mut run = fixture_run();
            finish_player(&mut run, &mut child.0).unwrap();
            assert_eq!(run.status, "incomplete-player-exited");
            assert!(run.exit_status.is_some());
            assert!(!run.terminated_by_harness);
        }
    }

    #[test]
    fn live_child_is_stopped_without_becoming_a_premature_exit() {
        let mut child = ManagedChild(
            Command::new(python())
                .args(["-c", "import time; time.sleep(30)"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut run = fixture_run();
        finish_player(&mut run, &mut child.0).unwrap();
        assert_eq!(run.status, "process-sampled-playback-unverified");
        assert!(run.exit_status.is_some());
        assert!(run.terminated_by_harness);
        assert!(child.0.try_wait().unwrap().is_some());
    }
}
