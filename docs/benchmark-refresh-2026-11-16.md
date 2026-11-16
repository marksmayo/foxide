# Hosted benchmark refresh — 2026-11-16

[Player Games run](https://github.com/marksmayo/foxide/actions/runs/43845858649) completed successfully on Windows Server 2025, macOS 15 ARM64 and Ubuntu 24.04, plus an isolated Linux X11 native UI session. The measured source commit is `80ef311bc7b13dcbe02003c9d10c1bbaebe0b3b8`.

## Fresh measured medals

These counts use only this run, not the older local fields retained in the main scoreboard. Each metric/scenario is an event; tied awards and related latency/resource observations mean medal totals are not an overall quality score.

| Field | Player | Gold | Silver | Bronze |
|---|---|---:|---:|---:|
| Windows hosted software playback | Foxide CLI | 2 | 9 | 3 |
| Windows hosted software playback | mpv | 9 | 3 | 2 |
| Windows hosted software playback | VLC | 4 | 1 | 4 |
| macOS hosted software playback | Foxide CLI | 2 | 8 | 4 |
| macOS hosted software playback | mpv | 8 | 4 | 2 |
| macOS hosted software playback | VLC | 4 | 2 | 3 |
| Linux hosted software playback | Foxide CLI | 2 | 8 | 4 |
| Linux hosted software playback | mpv | 12 | 2 | 0 |
| Linux hosted software playback | VLC | 1 | 3 | 5 |
| Linux native UI | Foxide Desktop | 12 | 8 | 4 |
| Linux native UI | VLC | 14 | 10 | 0 |
| Linux native UI | SMPlayer | 4 | 2 | 6 |

Foxide CLI wraps external mpv in the software cohort. This comparison does not measure Foxide Desktop decoding, physical GPU rendering, watts or battery life. Benchmark process-tree CPU and RSS are sampled over short eight-second trials with two seconds of warmup. Startup proxy measurements are engine readiness, not on-screen TTFF.

## Native menu acknowledgement medians

Ten fresh-process repeats per supported action; milliseconds include native input, accessibility queries and 10 ms observation polling.

| Menu | Foxide Desktop | VLC | SMPlayer |
|---|---:|---:|---:|
| Media / File | 90.34 | 144.13 | 330.35 |
| Playback | 103.35 | 139.65 | 330.44 |
| Audio | 83.29 | 135.20 | Unranked |
| Subtitles | 87.18 | 142.15 | Unranked |

All 100 attempted UI trials verified, producing 600 metric rows. Five unsupported action/adapter combinations remain unranked. Window sizes are application defaults, not equal video viewport sizes.

## Eligibility and documented features

Each playback platform verified 48 trials: 18 for Foxide, 18 for mpv, and 12 for VLC. Other installed players receive launch diagnostics without playback medals until a playback-state adapter verifies them. Null output cannot verify ASS composition. VLC AV1 software decoding and paused-state readiness did not satisfy the evidence gates in these runs; those cases remain visible without medals.

The catalog now contains 80 documented feature events across 18 entrants and 58 UI workload definitions (348 possible UI metric events; 393 total performance definitions). The 12 new workloads are adapter-required, not newly measured results. All sourced feature supporters share gold; platform, paid-tier and server requirements remain attached to claims. Unknown is not evidence of absence. Foxide has 23 documented feature golds per platform, separately from empirical medals.

The [feature and UI protocol](feature-and-ui-events.md) records research links and fixed-workload requirements. The [README](../README.md) and [scoreboard tables](scoreboards/scoreboards.md) contain the detailed per-platform results, graphs and evidence links. Their platform totals also include explicitly dated older local Windows fields; use the field tables or this summary for fresh hosted results alone.

## Validation

Three new catalog/evidence regression tests failed against the previous catalogs and passed after the changes. All 108 Python tests pass. Source-commit Rust checks, desktop builds, compositor regression and CodSpeed checks passed. The report job verified platform navigation and captured a rendered preview; the downloaded preview and published provenance were inspected.
