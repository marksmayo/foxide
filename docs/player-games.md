# Player Games protocol

The field contains Foxide and 17 reference players. It is representative, not a verified market-share top ten.

## Feature events

The versioned [feature catalog](../benchmarks/features.json) contains 30 events, official source links, research date, platform applicability, and notes. Every documented supporter receives gold, including any number of ties. Feature possession has no silver/bronze. Unknown, partial, and planned earn no medal and are not evidence of absence. Research depth differs; documented-gold counts do not establish an overall best product. Paid features and required components are noted.

Foxide's current medals explicitly identify external-mpv functionality. Engine ownership is not required, but the feature must be exposed by the application. A command/keyboard capability does not imply a custom GUI exists.

## Performance events

The [45-event catalog](../benchmarks/performance-events.json) defines startup, first audio, seeking, scrubbing, pause/resume, track switching, CPU/RAM in ten workloads, peak RAM, GPU utilization/allocation, power, energy, frame loss/pacing, A/V drift, I/O, wakeups, footprint, and decode throughput.

Performance requires at least three distinct verified repeats per entrant and two eligible entrants in one exact cohort. Every recorded trial must be valid; failed repeats cannot be silently discarded. Medians determine competition ranks: 1/1/3 receives gold/gold/bronze. Missing/non-finite values, negative costs, duplicate trials, mismatched profiles, and playback failures cannot medal. Zero is valid data.

Cohorts record OS, architecture, host, runner image, source hash, media SHA-256, scenario, duration, and warmup. Separate sessions/code changes have separate cohorts. Different OSes and hosts never share a performance podium.

## Hosted VM collection

`scripts/run_event_benchmarks.py` inventories every applicable native player. mpv, VLC, and Foxide use explicit software decode with null audio/video outputs. mpv/Foxide supply internal playback events. VLC supplies authenticated loopback HTTP state, decoded-frame counters, and advancing time. Steady sampling starts after playback readiness and the same warmup. A readiness timeout fails the case instead of reducing the CPU mean. Peak RSS includes startup and is a sampled peak.

Other frontends receive process-tree diagnostics with playback-unverified outcomes. An installed/running process, sign-in screen, or file-open dialog cannot earn playback medals. Native activation/control adapters remain required. Modern Media Player, Infuse, and unavailable packages retain their provisioning/adapter limitations.

A generated seed corpus is hash-verified and extended to 90 seconds of repeated synthetic bitstreams. HEVC performance files omit embedded subtitles; ASS uses one explicit external file. Null output disables native subtitle composition, so ASS results remain diagnostic/unranked. CPU is percent of one logical core. Tree RSS can double-count shared pages. Lua/HTTP observation overhead is included and differs by adapter.

Displayed TTFF, displayed seek latency, GPU residency, native frame pacing, and package power require physical collectors. They remain pending on hosted VMs. Internal playback-restart timing is a distinct proxy event. Idle, network, switching, scrubbing, footprint, and other unimplemented collectors remain visible instead of receiving guessed values.

## Run and publish

The **Player Games** Actions workflow installs/attempts each platform field, generates media, collects trials/diagnostics, uploads raw artifacts, and produces HTML/SVG scoreboards. It also runs on releases. The final job updates only README's generated region and `docs/scoreboards`; newer source commits prevent stale scoreboard pushes.

```powershell
python scripts/run_event_benchmarks.py --media-dir media/generated/games --output results/local/games-run --require-core
python scripts/build_medal_report.py --measurements results/local/games-run/measurements.json --output docs/scoreboards --readme README.md
```

`--plan` inventories/prepares without starting players. Raw logs preserve failures. Historical `results/windows-comparison` is an independent earlier snapshot and is not silently rewritten.
