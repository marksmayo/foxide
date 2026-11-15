# Player Games protocol

The field contains Foxide and 17 reference players. It is representative, not a verified market-share top ten.

## Feature events

The versioned [feature catalog](../benchmarks/features.json) contains 30 events, official source links, research date, platform applicability, and notes. Every documented supporter receives gold, including any number of ties. Feature possession has no silver/bronze. Unknown, partial, and planned earn no medal and are not evidence of absence. Research depth differs; documented-gold counts do not establish an overall best product. Paid features and required components are noted.

Foxide's current medals explicitly identify external-mpv functionality. Engine ownership is not required, but the feature must be exposed by the application. A command/keyboard capability does not imply a custom GUI exists.

## Performance events

The [117-event catalog](../benchmarks/performance-events.json) includes 72 UI-action events plus 45 playback/resource events covering startup, first audio, seeking, scrubbing, pause/resume, track switching, CPU/RAM in ten workloads, peak RAM, GPU utilization/allocation, power, energy, frame loss/pacing, A/V drift, I/O, wakeups, footprint, and decode throughput.

Performance requires at least three distinct verified repeats per entrant and two eligible entrants in one exact cohort. Every recorded trial must be valid; failed repeats cannot be silently discarded. Medians determine competition ranks: 1/1/3 receives gold/gold/bronze. Missing/non-finite values, negative costs, duplicate trials, mismatched profiles, and playback failures cannot medal. Zero is valid data.

Cohorts record OS, architecture, host, runner image, source hash, media SHA-256, scenario, duration, and warmup. Separate sessions/code changes have separate cohorts. Different OSes and hosts never share a performance podium.

## Hosted VM collection

`scripts/run_event_benchmarks.py` inventories every applicable native player. mpv, VLC, and Foxide use explicit software decode with null audio/video outputs. mpv/Foxide supply internal playback events. VLC supplies authenticated loopback HTTP state, decoded-frame counters, and advancing time. Steady sampling starts after playback readiness and the same warmup. A readiness timeout fails the case instead of reducing the CPU mean. Peak RSS includes startup and is a sampled peak.

Other frontends receive process-tree diagnostics with playback-unverified outcomes. An installed/running process, sign-in screen, or file-open dialog cannot earn playback medals. Native activation/control adapters remain required. Modern Media Player, Infuse, and unavailable packages retain their provisioning/adapter limitations.

A generated seed corpus is hash-verified and extended to 90 seconds of repeated synthetic bitstreams. HEVC performance files omit embedded subtitles; ASS uses one explicit external file. Null output disables native subtitle composition, so ASS results remain diagnostic/unranked. CPU is percent of one logical core. Tree RSS can double-count shared pages. Lua/HTTP observation overhead is included and differs by adapter.

Displayed TTFF, displayed seek latency, GPU residency, native frame pacing, and package power require physical collectors. They remain pending on hosted VMs. Internal playback-restart timing is a distinct proxy event. Idle, network, switching, scrubbing, footprint, and other unimplemented collectors remain visible instead of receiving guessed values.

## Run and publish

The **Player Games** Actions workflow installs/attempts each platform field, generates media, collects trials/diagnostics, uploads raw artifacts, and produces HTML/SVG scoreboards. It also runs on releases. Pushes to `bench/**` also run the workflow. Its final job commits the README, graphs and versioned evidence to that benchmark branch for PR review. It never writes directly to main; newer source commits prevent stale scoreboard pushes. Main/release runs publish artifacts.

```powershell
python scripts/run_event_benchmarks.py --media-dir media/generated/games --output results/local/games-run --require-core
python scripts/refresh_scoreboards.py --platform-dir results/local/games-run --output docs/scoreboards --readme README.md
```

`--plan` inventories/prepares without starting players. Raw logs preserve failures. Historical `results/windows-comparison` is an independent earlier snapshot and is not silently rewritten.

## Local desktop field and GitHub README

The Virtuoso-style README includes overall/performance/feature medal tables, a graph and score table per platform, and separate host/presentation fields with graphs for individual metrics and expandable raw median tables. A feature is counted once in the global feature total, even when supported on multiple platforms. Performance totals count eligible events in each independent field; they are not a combined speed index.

Build the desktop before collecting; do not run builds, tests or other benchmark sessions concurrently. Use a new output directory for each collection:

```powershell
python scripts/run_event_benchmarks.py --media-dir media/generated/comparison --output results/local/desktop-run --presentation native-window --players foxide mpv vlc --scenarios avc1080 hevc4k av14k flac speed2x paused
python scripts/refresh_scoreboards.py --platform-dir results/local/desktop-run --readme README.md
```

Native fields launch `foxide-desktop`, mpv and VLC with audio/video output. Software-null fields launch the existing Foxide CLI and disable hardware decoding/output. Native dimensions and audio routes use player defaults, so the native field is exploratory rather than a normalized presentation benchmark. Actual mpv/libmpv `hwdec-current` observations are preserved; `d3d11va-copy` must not be described as zero-copy. The desktop benchmark script is opt-in and loads before media; ordinary launches do not add it.

Trials run in a deterministic shuffled order (`--order-seed`), with at least three repeats. Hashes identify the collector, fixture and executable. Startup is an engine playback-restart proxy with uncontrolled cache state, never displayed TTFF. Eight-second resource samples are short workload comparisons, not the ten-minute power/battery protocol. Foreground/background activity is uncontrolled and hardware/display configuration is not normalized.

`benchmarks/results/` preserves the latest collection for each platform, hosted/local origin and presentation mode. Each bundle includes per-trial samples, player inventory, commands, playback verification, per-trial metrics and failure outcomes; source JSON pointers refer back to cases in the same versioned evidence. `refresh_scoreboards.py` combines these bundles with new sessions, retaining the original date/source/run for older fields. A newer failed collection replaces the old field rather than keeping old wins. Missing evidence cannot erase all results.

If hosted jobs cannot start, retain prior hosted evidence with its original date and mark the new attempt separately. Do not label historical macOS/Linux data as a fresh cross-platform run. Windows Media Player and other discovered frontends stay in the feature/runtime tables; process-only diagnostics earn no playback medals until a validated adapter confirms playback. UI-action and physical GPU/power collectors remain separate.

Post-collection installation checks live in `benchmarks/runtime/` as inventory-only supplements. The refresher uses them for runtime outcomes without replacing any measured collection or adding performance medals; any supplement asserting verified playback is rejected. This preserves the original pre-install benchmark inventory while allowing newly installed players to appear accurately in the report.
