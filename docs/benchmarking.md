# Foxide benchmarking protocol

## What is implemented

`player-bench run` directly launches mpv or VLC with a local file, starts a preinitialized standard-library Python process sampler, records the command and raw telemetry, repeats runs and creates JSON plus Markdown results. It isolates clean defaults from the tuned mpv hardware profile. `observe` samples an already running native app without terminating it. All unsupported metrics are nullable; incomplete runs are retained and excluded from the process summary. A successful process sample does not establish playback correctness.

The current runner measures sampled RSS/working set and process-tree CPU time. mpv additionally supplies a parent-observed initial playback-restart proxy, internal seek/restart proxies and decoder/output drop counters. The Lua instrumentation affects startup slightly; quantify it against uninstrumented runs. Its startup proxy includes file flush and up to roughly 5 ms polling delay plus scheduling. Seek proxies use a single mpv monotonic clock. Neither signal proves screen presentation. The runner has no display-TTFF, GPU/VRAM, package-power or actual frame-pacing collector yet. The commands below produce raw external telemetry for analysis; the runner does not automatically join those traces or certify their content.

## Install and inventory

On Windows:

```powershell
.\scripts\install-baselines.ps1 -Install
# Install portable CLI mpv using a build linked from https://mpv.io/installation/.
# FFmpeg and Python must be on PATH. Windows Performance Toolkit supplies WPR/WPA.
cargo build --release --locked
.\target\release\player-bench.exe doctor
& 'C:\Program Files\VideoLAN\VLC\vlc.exe' --version
& tools/mpv/mpv.exe --version
ffmpeg -version
Get-AppxPackage Microsoft.ZuneMusic | Select-Object Name,Version,PackageFullName
Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion
Get-CimInstance Win32_Processor | Select-Object Name,NumberOfLogicalProcessors
```

On macOS (with Homebrew installed), `brew install mpv ffmpeg` and `brew install --cask vlc iina`; QuickTime is supplied by macOS. Record `sw_vers`, `system_profiler SPHardwareDataType SPDisplaysDataType`, and `defaults read /Applications/IINA.app/Contents/Info CFBundleShortVersionString` plus the same for QuickTime Player. On Debian/Ubuntu, `sudo apt install mpv vlc ffmpeg linux-tools-common powertop`; select `linux-tools-$(uname -r)` for the running kernel where available. Record `uname -a`, `lscpu`, GPU/driver and `vainfo`/`nvidia-smi` when applicable. These package commands are setup instructions, not operations performed by the runner.

Keep a `session.json` alongside results containing machine ID; OS build; CPU/GPU; RAM; driver and player versions/build flags; executable SHA-256; actual installed footprint; media manifest hash; display resolution/refresh/HDR/brightness; audio route/volume; power source/plan; battery condition; thermal starting state; capture-tool version and flags. Modern Windows Media Player, legacy `wmplayer.exe`, and mpv.net are separate competitors. Do not substitute one for another.

## Fixtures and trial design

Generate smoke fixtures first; inspect their probes and play each to verify decoding, color/levels, sound and selected subtitles. Generate full fixtures only after encoders work. Use the exact same immutable local files for all competitors, on the same storage. The synthetic source is repeatable but not representative of every film; maintain a second legally sourced real-content corpus with hashes and licenses. See [media recipes](../media/README.md).

Use a 1080p window at a fixed display size for A and 4K/fullscreen for B/C if the display supports it. Keep identical scaling, display HDR, subtitle visibility, audio volume/output and graphics adapter. Use AC power with an unchanged plan for the first repeatability pass, then repeat on battery at a defined charge/brightness. Log thermal throttling, battery saver, variable refresh and background activity. Use ABBA or a seeded shuffled competitor order, at least five 10-minute power trials and 20 startup/seek trials. Cool back to a recorded temperature between power runs. Report every failed/unsupported trial, p50/p95 timings and uncertainty, not just a best run. Fixture D has no video TTFF: measure time-to-first-audio with a loopback/audio device collector separately, currently unavailable.

Warm start means a new player process after one unscored three-second priming run. The priming run does not guarantee every page in every dependency is resident; record the observed condition. A cold-reboot run requires a reboot and a preverified media/tool manifest. Perform one timed launch before other players, hashes, virus scans or file probing can warm the relevant pages. Repeat with additional reboots, randomized across competitors. Do not use a memory-allocation trick or a second launch as evidence of cold cache. The harness never purges system caches. `cold-reboot` is explicitly user-attested, not technically verified, and permits only one run per invocation.

```powershell
# After reboot, with fixtures/toolchain verified BEFORE reboot:
target/release/player-bench run --player mpv --exe tools/mpv/mpv.exe --media media/generated/full/A-avc-1080p.mp4 --cache cold-reboot --cache-evidence 'Reboot 09:00; first media/player access; manifest session-001' --repeat 1 --duration 10 --warmup 2

# Full power/process suite; run each competitor in balanced order across sessions:
.\scripts\benchmark-windows.ps1 -Player mpv -PlayerExecutable "$PWD\tools\mpv\mpv.exe"
.\scripts\benchmark-windows.ps1 -Player vlc -PlayerExecutable 'C:\Program Files\VideoLAN\VLC\vlc.exe'
```

```bash
export FOXIDE_PYTHON=python3
bash scripts/benchmark-unix.sh mpv "$(command -v mpv)" media/generated/full
# macOS VLC binary:
bash scripts/benchmark-unix.sh vlc /Applications/VLC.app/Contents/MacOS/VLC media/generated/full
# Linux VLC:
bash scripts/benchmark-unix.sh vlc "$(command -v vlc)" media/generated/full
```

## Startup and seeking: content-aware presentation

Time zero is the harness monotonic instant immediately before OS process spawn. Final TTFF requires the first displayed video-content frame, excluding splash/loading/black window presents. PresentMon records presentation events, not proof those events contain decoded video. Join a content-aware display observation to process creation and the selected swapchain, reject startup chrome, and save the raw evidence and clock calibration. Validate a subset with a high-speed camera/photodiode and a synchronized trigger. OS captures can change composition, HDR and power behavior; report the capture mode and calibrate its cost. Never compare a playback-restart proxy with a content-to-display number.

For each seek record the input/request monotonic timestamp, target PTS, keyframe/exact mode, first correct video-frame presentation and the next two presented content PTS values. Report first-correct-frame latency and subsequent stability separately. Require no stale frames after the new target. Two subsequent 60 Hz frames alone span about 33 ms, so the <30 ms goal applies to first correct presentation, not a requirement to wait for three displayed frames. Exact seeks depend on distance from the preceding keyframe; disclose the GOP and accept different exact/keyframe scores. For the scripted mpv test the first seek is +30 seconds with keyframe mode, followed by ten nonmonotonic exact positions at 10%, 80%, 25%, 65%, 35%, 90%, 15%, 70%, 45%, 55%. The file must exceed 31 seconds. The current script waits one second between completed seeks; it tests discrete exact seeks, not continuous pointer dragging. Add input-injected drag tests and separately measure thumbnail readiness.

```powershell
target/release/player-bench run --player mpv --exe tools/mpv/mpv.exe --media media/generated/full/A-avc-1080p.mp4 --cache warm --seeks --repeat 20 --duration 25 --warmup 2 --output results/local/seeks
```

VLC/native seeking is currently a documented manual procedure using the same positions and capture, not an automated cross-player seek adapter. Unsupported seek/capture paths remain N/A with a reason.

## Windows: ETW, GPU memory and native app

In separate consoles start a pinned [PresentMon console build](https://github.com/GameTechDev/PresentMon/blob/main/README-ConsoleApplication.md) **before** the timed launch. Use its own help to verify flags for that version:

```powershell
PresentMon.exe --process_name mpv.exe --output_file results/local/presents.csv --qpc_time --track_gpu_video --timed 645 --terminate_after_timed --no_console_stats
```

Capture process lifetime with ETW, correlate PID plus creation time, and retain QPC frequency. Do not derive TTFF from the first arbitrary Present. For diagnostics, check available [WPR profiles](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options), then capture one run:

```powershell
wpr -profiles
wpr -start GeneralProfile -filemode
# Run ONE benchmark, then stop this recording:
wpr -stop results/local/playback.etl
typeperf -qx 'GPU Process Memory'
typeperf -qx 'GPU Engine'
typeperf '\GPU Process Memory(*)\Dedicated Usage' '\GPU Process Memory(*)\Shared Usage' '\GPU Engine(*)\Utilization Percentage' -si 1 -sc 630 -f CSV -o results/local/gpu.csv
```

Counters may be localized or missing. Match engine/memory instances to the player PID lifetime; keep dedicated allocation and shared GPU memory distinct. Do not sum repeated engine counters as if they were independent GPU capacity. RSS includes CPU-resident pages; shared GPU allocations and RSS can overlap, so do not blindly add them. WPR/CPU utilization and Task Manager's power-usage category are not package Watts. Use a supported package energy sensor/ETW power provider with documented domain, or a calibrated external meter for system/battery power. PresentMon hardware power telemetry is device-specific and GPU power is not CPU package power.

For Modern Media Player, open the fixture through its UI, confirm playback and the true media process in Task Manager, then:

```powershell
Get-Process *Media* | Select-Object Id,ProcessName,Path,StartTime
target/release/player-bench observe --pid 1234 --label windows-media-player-modern --media media/generated/full/A-avc-1080p.mp4 --duration 630 --warmup 30
```

Replace 1234 with the verified PID. Do not sample `explorer.exe`, `ApplicationFrameHost.exe`, or a transient activation helper. This path measures steady playback only. A proper cold-start adapter needs packaged-app activation correlation and content-aware presentation; it is not implemented. Unsupported HEVC/MKV/TrueHD/subtitle combinations are compatibility failures, not zero-latency wins. Do not install paid codec extensions silently.

## macOS: native apps and power

```bash
open -a 'QuickTime Player' "$PWD/media/generated/full/A-avc-1080p.mp4"
pgrep -fl 'QuickTime Player'
target/release/player-bench observe --pid 1234 --label quicktime --media media/generated/full/A-avc-1080p.mp4 --duration 630 --warmup 30
open -a IINA "$PWD/media/generated/full/A-avc-1080p.mp4"
pgrep -fl '/IINA.app/Contents/MacOS/IINA'
# Repeat observe with IINA's verified GUI PID, not its CLI launcher.
powermetrics --help
# In a separate terminal; select only samplers available on this Mac:
sudo powermetrics --samplers cpu_power,gpu_power -i 1000 -n 630 -o results/local/powermetrics.txt
xcrun xctrace list templates
# If this template is available, record the already verified process for pacing:
xcrun xctrace record --template 'Metal System Trace' --attach 1234 --time-limit 630s --output results/local/playback.trace
```

`open` can activate an existing process, so its exit time is not app startup. Confirm the file is actually playing; QuickTime often opens documents paused. Some formats are unsupported. `powermetrics` samplers and package/GPU domains vary by hardware/macOS; preserve the help output and raw report. Activity Monitor Energy Impact is not Watts. Instruments traces identify presentation/GPU work; a content-aware observation is still needed for TTFF. These commands need local permissions and are not automatically elevated by the harness.

## Linux: perf, powercap and PowerTOP

```bash
perf list | rg -i 'energy|power'
# Availability and perf_event_paranoid permissions vary:
perf stat -a -e power/energy-pkg/ -I 1000 -o results/local/package-energy.txt -- sleep 630
sudo powertop --time=630 --csv=results/local/powertop.csv
# Inspect available powercap zones without changing limits or permissions:
find /sys/class/powercap -name name -o -name energy_uj -o -name max_energy_range_uj
```

For raw RAPL sampling use [powercap's cumulative `energy_uj` and wrap range](https://github.com/torvalds/linux/blob/master/Documentation/power/powercap/powercap.rst): package Watts = wrap-corrected delta microjoules / (1e6 × elapsed seconds). Sample each top-level package zone about once per second, handle wraps with `max_energy_range_uj`, and sum independent sockets only. Do not sum a package and its overlapping core/uncore subdomains. RAPL does not include every GPU/display/device and many machines deny access. PowerTOP attribution is an estimate; `--auto-tune` would change test conditions and must not be used. For NVIDIA, `nvidia-smi --query-gpu=timestamp,power.draw,utilization.gpu,memory.used --format=csv -l 1` supplies GPU-wide telemetry, not process/package power. Other drivers need their supported collectors.

Wayland/X11 presentation timing and compositor ownership differ. Capture compositor/presentation feedback where accessible and validate the content boundary externally. CPU `perf` events or mpv counters alone do not establish screen pacing. Linux and macOS sampler implementations are included but require validation on those operating systems; the initial host is Windows.

## Analysis, footprint and acceptance

Align collector clocks and PID lifetimes, discard only the defined warmup interval, and integrate energy over the exact remaining 600 seconds. Report raw idle and playback power plus their difference and domain. Do not attribute whole-package power to one process. Run a matched idle window with the same display/audio/collector configuration. Estimate confidence intervals from independent trials; require the interval on Foxide/native power ratio to fall within a predeclared tolerance (initial proposal ≤1.05) before claiming parity. Record battery runtime separately from package power.

Use decoder and output drop counters as separate engine metrics. Calculate display pacing variance/p95/p99 from content frame presentation intervals after removing deliberate repeat/redraw frames; diagnose compositor versus decode drops separately. A 60 fps file on a 60 Hz output should not be judged against a 120 Hz UI interval. Missing frames, statistics or supported decoder paths must stay missing/failed.

For footprint count the release executable, required shared libraries/frameworks/plugins, fonts and bundled resources once each. Record package download size and installed bytes separately; OS-provided framework bytes are documented separately. `Get-ChildItem <install-dir> -Recurse -File | Measure-Object Length -Sum`, `du -sk <app-dir>` and SHA-256 manifests are initial tools, with a per-product ownership list to exclude unrelated files. A small Rust executable does not establish a small media-player distribution.

The targets are hypotheses: cold display TTFF <120 ms; warm <50 ms; first correct seek frame <30 ms; 1080p steady RSS <80 MB (decimal); complete distribution <25 MB; native-equivalent power with no regressions in quality or drops. The runner reports MiB; convert explicitly before evaluating decimal-MB goals. Report peak and steady RSS separately. No target is currently met or failed by an engine-event proxy. Use [the scoreboard template](../results/comparison-template.md); incomplete measurements cannot become green release checks.
