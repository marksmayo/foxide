# Foxide

<!-- source-lines:start -->
![Lua source lines](https://img.shields.io/badge/Lua%20source%20lines-55-000080)
![PowerShell source lines](https://img.shields.io/badge/PowerShell%20source%20lines-206-5391fe)
![Python source lines](https://img.shields.io/badge/Python%20source%20lines-5170-3776ab)
![Rust source lines](https://img.shields.io/badge/Rust%20source%20lines-5055-dea584)
![Slint source lines](https://img.shields.io/badge/Slint%20source%20lines-209-7c3aed)
![TOML source lines](https://img.shields.io/badge/TOML%20source%20lines-152-9c4221)
![YAML source lines](https://img.shields.io/badge/YAML%20source%20lines-963-cb171e)

Physical source lines include comments and blank lines in crates/, scripts/, tests/, .github/, packaging/ and the root Cargo manifest. Vendored code, lockfiles, generated reports and build output are excluded. Refresh with python scripts/source_lines.py.
<!-- source-lines:end -->

<p align="center">
  <img src="crates/player-ui/assets/foxide.webp" alt="Foxide media player" width="420">
</p>

[![CI](https://github.com/marksmayo/foxide/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/marksmayo/foxide/actions/workflows/ci.yml)
[![Desktop builds](https://github.com/marksmayo/foxide/actions/workflows/desktop-ui.yml/badge.svg?branch=main)](https://github.com/marksmayo/foxide/actions/workflows/desktop-ui.yml)
[![Video regression](https://github.com/marksmayo/foxide/actions/workflows/video-render.yml/badge.svg?branch=main)](https://github.com/marksmayo/foxide/actions/workflows/video-render.yml)
[![Player Games](https://github.com/marksmayo/foxide/actions/workflows/player-games.yml/badge.svg?branch=main)](https://github.com/marksmayo/foxide/actions/workflows/player-games.yml)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/marksmayo/foxide?utm_source=badge)

[![Rust 1.99+](https://img.shields.io/badge/Rust-1.99%2B-dea584?logo=rust)](Cargo.toml)
[![Slint 1.18.1](https://img.shields.io/badge/Slint-1.18.1-7c3aed)](docs/desktop.md)
[![Target platforms](https://img.shields.io/badge/targets-Windows%20%7C%20macOS%20%7C%20Linux-0078d4)](docs/architecture.md)
[![Default Rust coverage gate](https://img.shields.io/badge/default_Rust_coverage_gate-%E2%89%A590%25-2ea44f)](docs/security-and-testing.md)
[![Policy and installer coverage gates](https://img.shields.io/badge/policy_%26_installer_gates-%E2%89%A590%25-2ea44f)](docs/security-and-testing.md)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Rust desktop media player with a Slint desktop preview and a transparent competitor scoreboard. The new desktop embeds libmpv in Foxide's own window; the existing `foxide` CLI still launches external mpv.

```powershell
cargo build -p player-ui --features desktop --bin foxide-desktop --release --locked
.\target\release\foxide-desktop.exe "C:\media\movie.mkv"
```

The desktop needs a libmpv ABI 2 library: Windows `tools/libmpv/libmpv-2.dll` or `FOXIDE_LIBMPV`, macOS `brew install mpv`, Linux your distribution's `libmpv2`. [Desktop setup and current limits](docs/desktop.md) · [UI action benchmarks](docs/ui-benchmarks.md). The UI includes menus, a playlist, file/URL opening, playback/speed/track controls, external subtitles, volume and fullscreen. Native playback, GPU interop and power acceptance remain in progress. Software-OpenGL CI uses a [pinned libmpv scaler-padding fix](third_party/mpv/README.md); downloaded/system libraries require separate runtime acceptance.

```powershell
.\target\release\foxide.exe "C:\media\movie.mkv"
.\target\release\foxide.exe --speed 1.5 --subtitle "C:\media\movie.srt" "C:\media\movie.mkv"
.\target\release\foxide.exe --autoqueue --resume-dir ".\results\local\resume" "C:\series\episode2.mkv"
```

Build with `cargo build --release --locked`. On Windows, `python scripts/install-mpv.py` installs a verified engine under `tools/mpv`. Playback controls include Space/pause, arrows/seek, Shift+arrows/exact seek, F/fullscreen, J/subtitles, #/audio track, S/screenshot, and Q/quit. Run `foxide --help` for tracks, offsets, tone mapping, picture controls, chapters, URLs, playlists, and window options.

[Performance implementation ledger](docs/performance-review/IMPLEMENTATION.md) · [Capabilities and remaining work](docs/features.md) · [Local players](docs/local-players.md) · [Benchmark protocol](docs/player-games.md) · [Architecture](docs/architecture.md) · [Actions](https://github.com/marksmayo/foxide/actions)

The **Player Games** workflow installs/attempts each platform's native field, records verified playback or labeled frontend diagnostics, produces the GitHub-native graphs, result tables, and expandable feature/metric details below; benchmark branches receive updates for PR review and downloadable HTML remains optional. Feature possession awards shared gold. Performance requires repeated, verified, comparable measurements. Hosted VMs have separate cohorts from physical GPU, battery, and displayed-frame tests.

Development follows red → green tests. CI enforces **90% Rust workspace line coverage** for default features and **90% coverage for medal/report policies and installers**, in separate gates. Feature-gated desktop/native rendering has a separate instrumented GPU CI report; the default-feature threshold does not cover it. [Measured coverage and security follow-up](docs/security-and-testing.md). A separate three-platform desktop workflow builds the GUI and renders a layout preview. Native PiP, thumbnails, drag-and-drop, persistent GUI resume, complete GUI coverage and physical GPU acceptance remain open.

CodSpeed tracks **233 CPU benchmark cases** across playback policy, UI actions, playlists and telemetry processing. [Suite and run commands](docs/codspeed.md) · [Case inventory](benchmarks/codspeed-inventory.json).

[Windows preview release packaging](docs/windows-releases.md) — automated tag builds, checksums and draft releases.

<!-- foxide-scores:start -->
## Benchmark

### Latest results

Every metric is a separate medal event within one platform, host, fixture and settings cohort. Performance requires ≥3 verified repeats and ≥2 comparable entrants. Documented feature support earns shared gold; unknown does not mean unsupported.

Overall feature gold counts each feature once across applicable platforms; performance totals count every eligible field. These totals are not an overall speed ranking.

**Overall**

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 78 | 22 | 7 | 107 |
| VLC | 58 | 18 | 20 | 96 |
| Foxide | 47 | 46 | 24 | 117 |
| IINA | 34 | 0 | 0 | 34 |
| Infuse | 22 | 0 | 0 | 22 |
| Kodi | 20 | 0 | 0 | 20 |
| SMPlayer | 19 | 2 | 6 | 27 |
| MPC-BE | 19 | 0 | 0 | 19 |
| MPC-HC | 15 | 0 | 0 | 15 |
| PotPlayer | 14 | 0 | 0 | 14 |
| KMPlayer | 13 | 0 | 0 | 13 |
| foobar2000 | 13 | 0 | 0 | 13 |
| GOM Player | 11 | 0 | 0 | 11 |
| Plex | 7 | 0 | 0 | 7 |
| QuickTime Player | 6 | 0 | 0 | 6 |
| FFplay | 5 | 0 | 0 | 5 |
| Windows Media Player | 4 | 0 | 0 | 4 |
| Windows Media Player Legacy | 4 | 0 | 0 | 4 |

**Performance**

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 41 | 22 | 7 | 70 |
| VLC | 33 | 18 | 20 | 71 |
| Foxide | 24 | 46 | 24 | 94 |
| SMPlayer | 4 | 2 | 6 | 12 |
| FFplay | 0 | 0 | 0 | 0 |
| GOM Player | 0 | 0 | 0 | 0 |
| IINA | 0 | 0 | 0 | 0 |
| Infuse | 0 | 0 | 0 | 0 |
| KMPlayer | 0 | 0 | 0 | 0 |
| Kodi | 0 | 0 | 0 | 0 |
| MPC-BE | 0 | 0 | 0 | 0 |
| MPC-HC | 0 | 0 | 0 | 0 |
| Plex | 0 | 0 | 0 | 0 |
| PotPlayer | 0 | 0 | 0 | 0 |
| QuickTime Player | 0 | 0 | 0 | 0 |
| Windows Media Player | 0 | 0 | 0 | 0 |
| Windows Media Player Legacy | 0 | 0 | 0 | 0 |
| foobar2000 | 0 | 0 | 0 | 0 |

**Features**

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 37 | 0 | 0 | 37 |
| IINA | 34 | 0 | 0 | 34 |
| VLC | 25 | 0 | 0 | 25 |
| Foxide | 23 | 0 | 0 | 23 |
| Infuse | 22 | 0 | 0 | 22 |
| Kodi | 20 | 0 | 0 | 20 |
| MPC-BE | 19 | 0 | 0 | 19 |
| MPC-HC | 15 | 0 | 0 | 15 |
| SMPlayer | 15 | 0 | 0 | 15 |
| PotPlayer | 14 | 0 | 0 | 14 |
| KMPlayer | 13 | 0 | 0 | 13 |
| foobar2000 | 13 | 0 | 0 | 13 |
| GOM Player | 11 | 0 | 0 | 11 |
| Plex | 7 | 0 | 0 | 7 |
| QuickTime Player | 6 | 0 | 0 | 6 |
| FFplay | 5 | 0 | 0 | 5 |
| Windows Media Player | 4 | 0 | 0 | 4 |
| Windows Media Player Legacy | 4 | 0 | 0 | 4 |

[Raw scores](docs/scoreboards/medals.json) · [Feature sources](docs/scoreboards/features.json) · [Performance protocols](docs/scoreboards/performance-events.json) · [Download optional HTML](docs/scoreboards/report.html)

Windowed fields use **Foxide Desktop**; software-only fields use the **Foxide CLI** launcher. Engine playback evidence is distinct from physical frame presentation. UI-action metrics, displayed TTFF, power and battery results remain pending unless explicitly measured.

### Windows

![Windows medal scores](docs/scoreboards/windows.svg)

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 58 | 16 | 5 | 79 |
| VLC | 39 | 3 | 12 | 54 |
| Foxide | 31 | 22 | 12 | 65 |
| Kodi | 20 | 0 | 0 | 20 |
| MPC-BE | 19 | 0 | 0 | 19 |
| MPC-HC | 15 | 0 | 0 | 15 |
| SMPlayer | 15 | 0 | 0 | 15 |
| PotPlayer | 14 | 0 | 0 | 14 |
| KMPlayer | 13 | 0 | 0 | 13 |
| foobar2000 | 13 | 0 | 0 | 13 |
| GOM Player | 11 | 0 | 0 | 11 |
| Plex | 7 | 0 | 0 | 7 |
| FFplay | 5 | 0 | 0 | 5 |
| Windows Media Player | 4 | 0 | 0 | 4 |
| Windows Media Player Legacy | 4 | 0 | 0 | 4 |

**Software decode · Foxide CLI · Hosted**

windows-hosted · Windows-2025Server-10.0.26100-SP0 · AMD64 · 4 logical CPUs · 2026-11-16T10:13:32.630591+00:00.

[Actions run](https://github.com/marksmayo/foxide/actions/runs/43845858649) · Source: `80ef311bc7b13dcbe02003c9d10c1bbaebe0b3b8`. Freshness is per field; older platform results are retained with their original collection dates.

Sampling: 8s after engine readiness, 2s warmup excluded; randomized order (seed 20261115); cache state uncontrolled.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 9 | 3 | 2 | 14 |
| VLC | 4 | 1 | 4 | 9 |
| Foxide | 2 | 9 | 3 | 14 |

![Windows: every measured metric](docs/scoreboards/windows-58f08bf147-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | mpv |
|---|---:|---:|---:|
| Engine playback restart latency | 76.2 ms | — | **71.9** ms |
| Paused video CPU | **0** % of one core | — | **0** % of one core |
| Paused video steady RAM | 103 MiB | — | **98.8** MiB |
| 1080p H.264 CPU | **26.7** % of one core | 28.9 % of one core | 31 % of one core |
| 1080p H.264 steady RAM | 122 MiB | 157 MiB | **118** MiB |
| 4K60 HEVC 10-bit CPU | 187 % of one core | **178** % of one core | 184 % of one core |
| 4K60 HEVC 10-bit steady RAM | 438 MiB | 911 MiB | **434** MiB |
| 4K AV1 CPU | 156 % of one core | — | **155** % of one core |
| 4K AV1 steady RAM | 542 MiB | — | **537** MiB |
| FLAC audio CPU | 1.06 % of one core | **0.26** % of one core | 0.79 % of one core |
| FLAC audio steady RAM | 55.8 MiB | **25.5** MiB | 51.7 MiB |
| 2× playback CPU | 58.8 % of one core | **54.8** % of one core | 61.1 % of one core |
| 2× playback steady RAM | 124 MiB | 158 MiB | **119** MiB |
| Peak playback RAM | 123 MiB | 158 MiB | **118** MiB |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Engine playback restart latency | `software-null-output/avc1080/a66ec10c23a67fd4` | 3, 3 |
| Paused video CPU | `software-null-output/paused/fb54dace11c80034` | 3, 3 |
| Paused video steady RAM | `software-null-output/paused/fb54dace11c80034` | 3, 3 |
| 1080p H.264 CPU | `software-null-output/avc1080/a66ec10c23a67fd4` | 3, 3, 3 |
| 1080p H.264 steady RAM | `software-null-output/avc1080/a66ec10c23a67fd4` | 3, 3, 3 |
| 4K60 HEVC 10-bit CPU | `software-null-output/hevc4k/bb6e407e62a0ba02` | 3, 3, 3 |
| 4K60 HEVC 10-bit steady RAM | `software-null-output/hevc4k/bb6e407e62a0ba02` | 3, 3, 3 |
| 4K AV1 CPU | `software-null-output/av14k/edcfc7d025788188` | 3, 3 |
| 4K AV1 steady RAM | `software-null-output/av14k/edcfc7d025788188` | 3, 3 |
| FLAC audio CPU | `software-null-output/flac/8564eb292ebfdefc` | 3, 3, 3 |
| FLAC audio steady RAM | `software-null-output/flac/8564eb292ebfdefc` | 3, 3, 3 |
| 2× playback CPU | `software-null-output/speed2x/3387a8d510ff993b` | 3, 3, 3 |
| 2× playback steady RAM | `software-null-output/speed2x/3387a8d510ff993b` | 3, 3, 3 |
| Peak playback RAM | `software-null-output/avc1080/a66ec10c23a67fd4` | 3, 3, 3 |

</details>


**Windowed playback · Foxide Desktop · Local**

windows-local · Windows-11-10.0.26300-SP0 · 20 logical CPUs · 2026-11-15T04:01:52.019159+00:00.

Source: `372de990b8f2810a32e091b01a553a1ec5d3a76f`. Freshness is per field; older platform results are retained with their original collection dates.

Sampling: 8s after engine readiness, 2s warmup excluded; randomized order (seed 20261115); cache state uncontrolled.

Exploratory native playback at each player’s default window size and audio route; dimensions are not normalized. Hardware fallback and observer overhead differ. This field does not establish displayed FPS or battery efficiency.

| Observed decoder | Foxide | VLC | mpv |
|---|---|---|---|
| 1080p H.264 | d3d11va-copy | Not reported | d3d11va |
| 4K60 HEVC | d3d11va-copy | Not reported | d3d11va |
| 4K60 AV1 | d3d11va-copy | Not reported | d3d11va |

Engine observations, not scanout proof. The [mpv manual](https://mpv.io/manual/stable/#options-hwdec) identifies <code>d3d11va-copy</code> as copying decoded video into system RAM; this is not zero-copy.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 7 | 6 | 1 | 14 |
| Foxide | 4 | 6 | 4 | 14 |
| VLC | 3 | 2 | 6 | 11 |

![Windows: every measured metric](docs/scoreboards/windows-f9366948d9-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | mpv |
|---|---:|---:|---:|
| Engine playback restart latency | 4.36e+03 ms | — | **3.16e+03** ms |
| Paused video CPU | 0.256 % of one core | — | **0** % of one core |
| Paused video steady RAM | 242 MiB | — | **229** MiB |
| 1080p H.264 CPU | **9.39** % of one core | 9.75 % of one core | 9.49 % of one core |
| 1080p H.264 steady RAM | 259 MiB | 379 MiB | **239** MiB |
| 4K60 HEVC 10-bit CPU | 101 % of one core | **11.9** % of one core | 21.4 % of one core |
| 4K60 HEVC 10-bit steady RAM | **1.17e+03** MiB | 1.56e+03 MiB | 1.2e+03 MiB |
| 4K AV1 CPU | 87.5 % of one core | **15.1** % of one core | 24.3 % of one core |
| 4K AV1 steady RAM | **801** MiB | 820 MiB | 803 MiB |
| FLAC audio CPU | 4.14 % of one core | 2.31 % of one core | **1.31** % of one core |
| FLAC audio steady RAM | 156 MiB | **65.9** MiB | 150 MiB |
| 2× playback CPU | **10.5** % of one core | 10.5 % of one core | 21.9 % of one core |
| 2× playback steady RAM | 264 MiB | 365 MiB | **242** MiB |
| Peak playback RAM | 260 MiB | 379 MiB | **245** MiB |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Engine playback restart latency | `native-default-window/avc1080/e5a3127a89486db7` | 3, 3 |
| Paused video CPU | `native-default-window/paused/4fe317a2a7ce449c` | 3, 3 |
| Paused video steady RAM | `native-default-window/paused/4fe317a2a7ce449c` | 3, 3 |
| 1080p H.264 CPU | `native-default-window/avc1080/e5a3127a89486db7` | 3, 3, 3 |
| 1080p H.264 steady RAM | `native-default-window/avc1080/e5a3127a89486db7` | 3, 3, 3 |
| 4K60 HEVC 10-bit CPU | `native-default-window/hevc4k/2e513aff25627cca` | 3, 3, 3 |
| 4K60 HEVC 10-bit steady RAM | `native-default-window/hevc4k/2e513aff25627cca` | 3, 3, 3 |
| 4K AV1 CPU | `native-default-window/av14k/50e9d1024fa3f2f7` | 3, 3, 3 |
| 4K AV1 steady RAM | `native-default-window/av14k/50e9d1024fa3f2f7` | 3, 3, 3 |
| FLAC audio CPU | `native-default-window/flac/3a05bf63a227e2b5` | 3, 3, 3 |
| FLAC audio steady RAM | `native-default-window/flac/3a05bf63a227e2b5` | 3, 3, 3 |
| 2× playback CPU | `native-default-window/speed2x/4c72021d4bdad5b6` | 3, 3, 3 |
| 2× playback steady RAM | `native-default-window/speed2x/4c72021d4bdad5b6` | 3, 3, 3 |
| Peak playback RAM | `native-default-window/avc1080/e5a3127a89486db7` | 3, 3, 3 |

</details>


**Software decode · Foxide CLI · Local**

windows-local · Windows-11-10.0.26300-SP0 · 20 logical CPUs · 2026-11-15T04:29:30.454755+00:00.

Source: `188f2e6b49b5d638b7f641b47d13ae0ee6906740`. Freshness is per field; older platform results are retained with their original collection dates.

Sampling: 8s after engine readiness, 2s warmup excluded; randomized order (seed 20261115); cache state uncontrolled.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| VLC | 7 | 0 | 2 | 9 |
| mpv | 5 | 7 | 2 | 14 |
| Foxide | 2 | 7 | 5 | 14 |

![Windows: every measured metric](docs/scoreboards/windows-0157c4dec0-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | mpv |
|---|---:|---:|---:|
| Engine playback restart latency | **2.35e+03** ms | — | 2.42e+03 ms |
| Paused video CPU | 0.325 % of one core | — | **0.27** % of one core |
| Paused video steady RAM | 175 MiB | — | **170** MiB |
| 1080p H.264 CPU | 70.9 % of one core | **62.5** % of one core | 64.5 % of one core |
| 1080p H.264 steady RAM | 200 MiB | **166** MiB | 196 MiB |
| 4K60 HEVC 10-bit CPU | 216 % of one core | — | **207** % of one core |
| 4K60 HEVC 10-bit steady RAM | **835** MiB | — | 848 MiB |
| 4K AV1 CPU | 72.3 % of one core | **51** % of one core | 74.9 % of one core |
| 4K AV1 steady RAM | 643 MiB | 698 MiB | **640** MiB |
| FLAC audio CPU | 1.18 % of one core | 2.68 % of one core | **1.15** % of one core |
| FLAC audio steady RAM | 58.7 MiB | **26.2** MiB | 53.7 MiB |
| 2× playback CPU | 127 % of one core | **47** % of one core | 138 % of one core |
| 2× playback steady RAM | 205 MiB | **174** MiB | 201 MiB |
| Peak playback RAM | 201 MiB | **176** MiB | 198 MiB |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Engine playback restart latency | `software-null-output/avc1080/627de1d25dff605b` | 3, 3 |
| Paused video CPU | `software-null-output/paused/daf715b1bd930d72` | 3, 3 |
| Paused video steady RAM | `software-null-output/paused/daf715b1bd930d72` | 3, 3 |
| 1080p H.264 CPU | `software-null-output/avc1080/627de1d25dff605b` | 3, 3, 3 |
| 1080p H.264 steady RAM | `software-null-output/avc1080/627de1d25dff605b` | 3, 3, 3 |
| 4K60 HEVC 10-bit CPU | `software-null-output/hevc4k/bef5bd64c814bebc` | 3, 3 |
| 4K60 HEVC 10-bit steady RAM | `software-null-output/hevc4k/bef5bd64c814bebc` | 3, 3 |
| 4K AV1 CPU | `software-null-output/av14k/4c0374fd24a77a8b` | 3, 3, 3 |
| 4K AV1 steady RAM | `software-null-output/av14k/4c0374fd24a77a8b` | 3, 3, 3 |
| FLAC audio CPU | `software-null-output/flac/243ea89a8ffb1188` | 3, 3, 3 |
| FLAC audio steady RAM | `software-null-output/flac/243ea89a8ffb1188` | 3, 3, 3 |
| 2× playback CPU | `software-null-output/speed2x/f035ebb00f869495` | 3, 3, 3 |
| 2× playback steady RAM | `software-null-output/speed2x/f035ebb00f869495` | 3, 3, 3 |
| Peak playback RAM | `software-null-output/avc1080/627de1d25dff605b` | 3, 3, 3 |

</details>

<details><summary>Runtime outcomes and feature events</summary>

| Player | Runtime outcome | Evidence / limitation |
|---|---|---|
| Foxide | Verified playback 54/57 | sampled-playback-unverified, verified-playback |
| VLC | Verified playback 41/57 | playback-readiness-timeout, sampled-playback-unverified, verified-playback |
| mpv | Verified playback 54/57 | sampled-playback-unverified, verified-playback |
| Windows Media Player | install-blocked | Local Store package was not registered; WinGet list/install both returned 0x8A150001. Store desktop control is unavailable. Interactive Store installation is pending. |
| Windows Media Player Legacy | Unverified / diagnostic (6 cases) | early-exit, sampled-playback-unverified |
| MPC-HC | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| MPC-BE | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| PotPlayer | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| SMPlayer | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| KMPlayer | Unverified / diagnostic (3 cases) | early-exit, sampled-playback-unverified |
| GOM Player | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| FFplay | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| Kodi | Unverified / diagnostic (6 cases) | sampled-playback-unverified |
| Plex | Unverified / diagnostic (6 cases) | early-exit, sampled-playback-unverified |
| foobar2000 | Unverified / diagnostic (6 cases) | sampled-playback-unverified |

| Feature | Foxide | VLC | mpv | Windows Media Player | Windows Media Player Legacy | MPC-HC | MPC-BE | PotPlayer | SMPlayer | KMPlayer | GOM Player | FFplay | Kodi | Plex | foobar2000 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? | N/A |
| Audio track selection | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 | 🥇 | N/A |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | N/A |
| External subtitle files | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A |
| Variable playback speed | 🥇 | 🥇 | 🥇 | ? | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? |
| Playlists | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | N/A |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | N/A |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | N/A |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | ? | ? | 🥇 | ? | N/A |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | 🥇 | ? | 🥇 | ? | N/A |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | N/A |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Native picture in picture | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 |
| Persistent playback resume | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | N/A |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | N/A |
| Chromecast video output | ? | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | N/A |
| AirPlay video output | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| AirPlay audio receiver | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Search and download subtitles | ? | ? | ? | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | N/A |
| Video-site URL playback | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | 🥇 | 🥇 | ? | ? | ? | ? | N/A |
| User extension or scripting system | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | 🥇 |
| Editable keyboard bindings | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| Customizable UI layout or skins | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | 🥇 |
| Touch or trackpad playback gestures | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? |
| System media-control integration | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Compact dedicated music view | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Dark interface theme | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Music playback artwork view | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Automatic movie and TV library metadata | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | N/A |
| Browse SMB network media | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Direct cloud-storage playback | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Download server media for offline viewing | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Cross-device playback progress sync | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Live TV and guide frontend | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | N/A |
| Schedule TV recordings from the player | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | N/A |
| HTTP remote-control interface | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Documented JSON IPC control | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| HD audio bitstream passthrough | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| 360-degree panoramic video | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | 🥇 | 🥇 | ? | ? | ? | N/A |
| Stereoscopic 3D video | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | N/A |
| Capture playing video to a clip | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | N/A |
| Desktop screen recording | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Trim and rearrange video clips | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Audio conversion or transcoding | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| Audio CD ripping | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| Audio or data CD burning | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Edit media tags | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| Built-in internet-radio directory | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| Video rotation and flip controls | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| ICC display color management | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| User GPU shader pipeline | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Debanding control | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Single-frame advance | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | N/A |
| Playlist search and filtering | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? |
| Two simultaneous subtitle tracks | ? | ? | 🥇 | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | N/A |
| Browse UPnP / DLNA media servers | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Share a library as a UPnP server | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Game controller UI navigation | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Browse WebDAV media shares | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Browse NFS media shares | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Trakt watched-history scrobbling | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Native Plex server library integration | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | N/A |
| Native Emby server library integration | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Native Jellyfin server library integration | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Skip detected episode intros | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | N/A |
| Music visualization display | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? |

? = undocumented/unverified. N/A = inapplicable media type.

</details>

### macOS

![macOS medal scores](docs/scoreboards/macos.svg)

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 45 | 4 | 2 | 51 |
| IINA | 34 | 0 | 0 | 34 |
| VLC | 29 | 2 | 3 | 34 |
| Foxide | 25 | 8 | 4 | 37 |
| Infuse | 22 | 0 | 0 | 22 |
| Kodi | 20 | 0 | 0 | 20 |
| SMPlayer | 14 | 0 | 0 | 14 |
| foobar2000 | 12 | 0 | 0 | 12 |
| Plex | 7 | 0 | 0 | 7 |
| QuickTime Player | 6 | 0 | 0 | 6 |
| FFplay | 5 | 0 | 0 | 5 |

**Software decode · Foxide CLI · Hosted**

macos-hosted · macOS-15.8.2-arm64-arm-64bit · arm64 · 3 logical CPUs · 2026-11-16T10:11:49.021719+00:00.

[Actions run](https://github.com/marksmayo/foxide/actions/runs/43845858649) · Source: `80ef311bc7b13dcbe02003c9d10c1bbaebe0b3b8`. Freshness is per field; older platform results are retained with their original collection dates.

Sampling: 8s after engine readiness, 2s warmup excluded; randomized order (seed 20261115); cache state uncontrolled.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 8 | 4 | 2 | 14 |
| VLC | 4 | 2 | 3 | 9 |
| Foxide | 2 | 8 | 4 | 14 |

![macOS: every measured metric](docs/scoreboards/macos-ea5d3184dd-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | mpv |
|---|---:|---:|---:|
| Engine playback restart latency | 219 ms | — | **176** ms |
| Paused video CPU | 0.622 % of one core | — | **0.331** % of one core |
| Paused video steady RAM | 151 MiB | — | **148** MiB |
| 1080p H.264 CPU | 47 % of one core | 45.9 % of one core | **29.2** % of one core |
| 1080p H.264 steady RAM | **192** MiB | 200 MiB | 196 MiB |
| 4K60 HEVC 10-bit CPU | 205 % of one core | **165** % of one core | 206 % of one core |
| 4K60 HEVC 10-bit steady RAM | 444 MiB | 885 MiB | **442** MiB |
| 4K AV1 CPU | **110** % of one core | — | 111 % of one core |
| 4K AV1 steady RAM | 550 MiB | — | **540** MiB |
| FLAC audio CPU | 4.22 % of one core | **1.53** % of one core | 3.77 % of one core |
| FLAC audio steady RAM | 158 MiB | **67.4** MiB | 150 MiB |
| 2× playback CPU | 68.5 % of one core | **57** % of one core | 70.2 % of one core |
| 2× playback steady RAM | 196 MiB | 200 MiB | **194** MiB |
| Peak playback RAM | 213 MiB | 200 MiB | **196** MiB |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Engine playback restart latency | `software-null-output/avc1080/2b35c1bec079081b` | 3, 3 |
| Paused video CPU | `software-null-output/paused/b88fdf47ebb54567` | 3, 3 |
| Paused video steady RAM | `software-null-output/paused/b88fdf47ebb54567` | 3, 3 |
| 1080p H.264 CPU | `software-null-output/avc1080/2b35c1bec079081b` | 3, 3, 3 |
| 1080p H.264 steady RAM | `software-null-output/avc1080/2b35c1bec079081b` | 3, 3, 3 |
| 4K60 HEVC 10-bit CPU | `software-null-output/hevc4k/161a4da04b64241d` | 3, 3, 3 |
| 4K60 HEVC 10-bit steady RAM | `software-null-output/hevc4k/161a4da04b64241d` | 3, 3, 3 |
| 4K AV1 CPU | `software-null-output/av14k/f3667f35c9c606d2` | 3, 3 |
| 4K AV1 steady RAM | `software-null-output/av14k/f3667f35c9c606d2` | 3, 3 |
| FLAC audio CPU | `software-null-output/flac/5681591eb6003d19` | 3, 3, 3 |
| FLAC audio steady RAM | `software-null-output/flac/5681591eb6003d19` | 3, 3, 3 |
| 2× playback CPU | `software-null-output/speed2x/7e187283375514d5` | 3, 3, 3 |
| 2× playback steady RAM | `software-null-output/speed2x/7e187283375514d5` | 3, 3, 3 |
| Peak playback RAM | `software-null-output/avc1080/2b35c1bec079081b` | 3, 3, 3 |

</details>

<details><summary>Runtime outcomes and feature events</summary>

| Player | Runtime outcome | Evidence / limitation |
|---|---|---|
| Foxide | Verified playback 18/21 | sampled-playback-unverified, verified-playback |
| VLC | Verified playback 12/21 | playback-readiness-timeout, sampled-playback-unverified, verified-playback |
| mpv | Verified playback 18/21 | sampled-playback-unverified, verified-playback |
| SMPlayer | not-detected | No native executable found; packaged-app/account/Store activation may need a dedicated adapter. |
| FFplay | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| Kodi | Unverified / diagnostic (3 cases) | early-exit |
| Plex | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| foobar2000 | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| Infuse | not-detected | No native executable found; packaged-app/account/Store activation may need a dedicated adapter. |
| IINA | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| QuickTime Player | Unverified / diagnostic (3 cases) | sampled-playback-unverified |

| Feature | Foxide | VLC | mpv | SMPlayer | FFplay | Kodi | Plex | foobar2000 | Infuse | IINA | QuickTime Player |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | N/A | 🥇 | 🥇 | ? |
| Audio track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | N/A | 🥇 | 🥇 | ? |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | N/A | 🥇 | 🥇 | 🥇 |
| External subtitle files | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | 🥇 | 🥇 | ? |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| Variable playback speed | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 |
| Playlists | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | 🥇 | 🥇 | ? | 🥇 | ? |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | ? | 🥇 | ? |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | ? | 🥇 | ? |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | N/A | ? | 🥇 | ? |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | ? | 🥇 | 🥇 | ? |
| Native picture in picture | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | 🥇 | 🥇 |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Persistent playback resume | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | 🥇 | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | ? | ? | ? | 🥇 | 🥇 | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| Chromecast video output | ? | 🥇 | ? | 🥇 | ? | ? | ? | N/A | ? | ? | ? |
| AirPlay video output | ? | ? | ? | ? | ? | ? | ? | N/A | ? | ? | 🥇 |
| AirPlay audio receiver | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |
| Search and download subtitles | ? | ? | ? | 🥇 | ? | ? | ? | N/A | ? | 🥇 | ? |
| Video-site URL playback | ? | ? | 🥇 | 🥇 | ? | ? | ? | N/A | ? | ? | ? |
| User extension or scripting system | ? | ? | 🥇 | ? | ? | 🥇 | ? | 🥇 | ? | 🥇 | ? |
| Editable keyboard bindings | ? | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? |
| Customizable UI layout or skins | ? | ? | ? | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Touch or trackpad playback gestures | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| System media-control integration | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Compact dedicated music view | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Dark interface theme | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Music playback artwork view | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Automatic movie and TV library metadata | ? | ? | ? | ? | ? | 🥇 | ? | N/A | 🥇 | ? | ? |
| Browse SMB network media | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |
| Direct cloud-storage playback | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | ? | ? |
| Download server media for offline viewing | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? |
| Cross-device playback progress sync | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | ? | ? |
| Live TV and guide frontend | ? | ? | ? | ? | ? | 🥇 | ? | N/A | ? | ? | ? |
| Schedule TV recordings from the player | ? | ? | ? | ? | ? | 🥇 | ? | N/A | ? | ? | ? |
| HTTP remote-control interface | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |
| Documented JSON IPC control | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? |
| HD audio bitstream passthrough | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? |
| 360-degree panoramic video | ? | 🥇 | ? | ? | ? | ? | ? | N/A | ? | ? | ? |
| Stereoscopic 3D video | ? | ? | ? | ? | ? | ? | ? | N/A | ? | ? | ? |
| Capture playing video to a clip | ? | ? | ? | ? | ? | ? | ? | N/A | ? | ? | ? |
| Desktop screen recording | ? | 🥇 | ? | ? | ? | ? | ? | N/A | ? | ? | ? |
| Trim and rearrange video clips | ? | ? | ? | ? | ? | ? | ? | N/A | ? | ? | 🥇 |
| Audio conversion or transcoding | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? |
| Audio CD ripping | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Audio or data CD burning | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Edit media tags | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? |
| Built-in internet-radio directory | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? |
| Video rotation and flip controls | ? | ? | 🥇 | ? | ? | ? | ? | N/A | ? | ? | 🥇 |
| ICC display color management | ? | ? | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| User GPU shader pipeline | ? | ? | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| Debanding control | ? | ? | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| Single-frame advance | ? | ? | 🥇 | ? | 🥇 | ? | ? | N/A | ? | ? | ? |
| Playlist search and filtering | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Two simultaneous subtitle tracks | ? | ? | 🥇 | 🥇 | ? | ? | ? | N/A | ? | ? | ? |
| Browse UPnP / DLNA media servers | ? | ? | ? | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? |
| Share a library as a UPnP server | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |
| Game controller UI navigation | ? | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |
| Browse WebDAV media shares | ? | ? | ? | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? |
| Browse NFS media shares | ? | ? | ? | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? |
| Trakt watched-history scrobbling | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | ? | ? |
| Native Plex server library integration | ? | ? | ? | ? | ? | ? | 🥇 | N/A | 🥇 | ? | ? |
| Native Emby server library integration | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | ? | ? |
| Native Jellyfin server library integration | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | ? | ? |
| Skip detected episode intros | ? | ? | ? | ? | ? | ? | 🥇 | N/A | 🥇 | ? | ? |
| Music visualization display | ? | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? |

? = undocumented/unverified. N/A = inapplicable media type.

</details>

### Linux

![Linux medal scores](docs/scoreboards/linux.svg)

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 49 | 2 | 0 | 51 |
| VLC | 40 | 13 | 5 | 58 |
| Foxide | 37 | 16 | 8 | 61 |
| Kodi | 20 | 0 | 0 | 20 |
| SMPlayer | 18 | 2 | 6 | 26 |
| Plex | 7 | 0 | 0 | 7 |
| FFplay | 5 | 0 | 0 | 5 |

**Native UI actions · default application windows · Hosted**

linux-hosted · 2026-11-16T10:14:43.101179+00:00.

[Actions run](https://github.com/marksmayo/foxide/actions/runs/43845858649) · Source: `80ef311bc7b13dcbe02003c9d10c1bbaebe0b3b8`. Freshness is per field; older platform results are retained with their original collection dates.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| VLC | 14 | 10 | 0 | 24 |
| Foxide | 12 | 8 | 4 | 24 |
| SMPlayer | 4 | 2 | 6 | 12 |

![Linux: every measured metric](docs/scoreboards/linux-c0ce56f269-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | SMPlayer |
|---|---:|---:|---:|
| Media/File menu — UI state latency | **90.3** ms | 144 ms | 330 ms |
| Media/File menu — Action CPU time | 90 ms | **65** ms | 140 ms |
| Media/File menu — Post-action RSS | 191 MiB | **96.2** MiB | 96.6 MiB |
| Media/File menu — Native input dispatch | **0.22** ms | **0.189** ms | **0.201** ms |
| Media/File menu — UI state observation after dispatch | **90.1** ms | 144 ms | 330 ms |
| Media/File menu — Action RSS increase | 2.38 MiB | 1.12 MiB | **0.188** MiB |
| Playback menu — UI state latency | **103** ms | 140 ms | 330 ms |
| Playback menu — Action CPU time | 120 ms | **60** ms | 150 ms |
| Playback menu — Post-action RSS | 189 MiB | **96.2** MiB | 96.8 MiB |
| Playback menu — Native input dispatch | **0.213** ms | **0.189** ms | **0.199** ms |
| Playback menu — UI state observation after dispatch | **103** ms | 139 ms | 330 ms |
| Playback menu — Action RSS increase | 3.78 MiB | 1.06 MiB | **0.5** MiB |
| Audio menu — UI state latency | **83.3** ms | 135 ms | — |
| Audio menu — Action CPU time | 70 ms | **60** ms | — |
| Audio menu — Post-action RSS | 187 MiB | **95.9** MiB | — |
| Audio menu — Native input dispatch | **0.244** ms | **0.177** ms | — |
| Audio menu — UI state observation after dispatch | **82.7** ms | 135 ms | — |
| Audio menu — Action RSS increase | 1.58 MiB | **0.875** MiB | — |
| Subtitle menu — UI state latency | **87.2** ms | 142 ms | — |
| Subtitle menu — Action CPU time | 80 ms | **60** ms | — |
| Subtitle menu — Post-action RSS | 186 MiB | **95.6** MiB | — |
| Subtitle menu — Native input dispatch | **0.336** ms | **0.207** ms | — |
| Subtitle menu — UI state observation after dispatch | **86.8** ms | 142 ms | — |
| Subtitle menu — Action RSS increase | 1.59 MiB | **0.648** MiB | — |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Media/File menu — UI state latency | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Media/File menu — Action CPU time | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Media/File menu — Post-action RSS | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Media/File menu — Native input dispatch | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Media/File menu — UI state observation after dispatch | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Media/File menu — Action RSS increase | `native-ui/menu_media/1943adb5dfbdfbd6` | 10, 10, 10 |
| Playback menu — UI state latency | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Playback menu — Action CPU time | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Playback menu — Post-action RSS | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Playback menu — Native input dispatch | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Playback menu — UI state observation after dispatch | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Playback menu — Action RSS increase | `native-ui/menu_playback/a39ce9201910ff00` | 10, 10, 10 |
| Audio menu — UI state latency | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Audio menu — Action CPU time | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Audio menu — Post-action RSS | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Audio menu — Native input dispatch | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Audio menu — UI state observation after dispatch | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Audio menu — Action RSS increase | `native-ui/menu_audio/2d4d4fccd3a22abb` | 10, 10 |
| Subtitle menu — UI state latency | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |
| Subtitle menu — Action CPU time | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |
| Subtitle menu — Post-action RSS | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |
| Subtitle menu — Native input dispatch | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |
| Subtitle menu — UI state observation after dispatch | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |
| Subtitle menu — Action RSS increase | `native-ui/menu_subtitles/e22c8bbd939ada0c` | 10, 10 |

</details>


**Software decode · Foxide CLI · Hosted**

linux-hosted · Linux-7.0.0-1014-aws-x86_64-with-glibc2.39 · x86_64 · 4 logical CPUs · 2026-11-16T10:10:28.239948+00:00.

[Actions run](https://github.com/marksmayo/foxide/actions/runs/43845858649) · Source: `80ef311bc7b13dcbe02003c9d10c1bbaebe0b3b8`. Freshness is per field; older platform results are retained with their original collection dates.

Sampling: 8s after engine readiness, 2s warmup excluded; randomized order (seed 20261115); cache state uncontrolled.

| Player | 🥇 | 🥈 | 🥉 | Total |
|---|---:|---:|---:|---:|
| mpv | 12 | 2 | 0 | 14 |
| Foxide | 2 | 8 | 4 | 14 |
| VLC | 1 | 3 | 5 | 9 |

![Linux: every measured metric](docs/scoreboards/linux-efcd21edc8-scenarios.svg)

<details><summary>Every measured metric — median values and podiums</summary>

| Metric | Foxide | VLC | mpv |
|---|---:|---:|---:|
| Engine playback restart latency | 91.8 ms | — | **90.6** ms |
| Paused video CPU | **0** % of one core | — | **0** % of one core |
| Paused video steady RAM | 115 MiB | — | **112** MiB |
| 1080p H.264 CPU | 27.8 % of one core | 27.5 % of one core | **27.5** % of one core |
| 1080p H.264 steady RAM | 134 MiB | 181 MiB | **131** MiB |
| 4K60 HEVC 10-bit CPU | 204 % of one core | 203 % of one core | **199** % of one core |
| 4K60 HEVC 10-bit steady RAM | 469 MiB | 955 MiB | **466** MiB |
| 4K AV1 CPU | **152** % of one core | — | 152 % of one core |
| 4K AV1 steady RAM | 548 MiB | — | **545** MiB |
| FLAC audio CPU | 0.49 % of one core | 0.685 % of one core | **0.327** % of one core |
| FLAC audio steady RAM | 67.9 MiB | **29.5** MiB | 65.1 MiB |
| 2× playback CPU | 55.5 % of one core | 55.3 % of one core | **55.2** % of one core |
| 2× playback steady RAM | 135 MiB | 180 MiB | **132** MiB |
| Peak playback RAM | 134 MiB | 181 MiB | **131** MiB |

Bold values are gold medians. — means no eligible comparable result. Trial evidence and exact cohort IDs:

| Metric | Cohort | Trials per eligible entrant |
|---|---|---:|
| Engine playback restart latency | `software-null-output/avc1080/2d35314f3fa91805` | 3, 3 |
| Paused video CPU | `software-null-output/paused/323a6608a4239a28` | 3, 3 |
| Paused video steady RAM | `software-null-output/paused/323a6608a4239a28` | 3, 3 |
| 1080p H.264 CPU | `software-null-output/avc1080/2d35314f3fa91805` | 3, 3, 3 |
| 1080p H.264 steady RAM | `software-null-output/avc1080/2d35314f3fa91805` | 3, 3, 3 |
| 4K60 HEVC 10-bit CPU | `software-null-output/hevc4k/5340f1b98f6b23e0` | 3, 3, 3 |
| 4K60 HEVC 10-bit steady RAM | `software-null-output/hevc4k/5340f1b98f6b23e0` | 3, 3, 3 |
| 4K AV1 CPU | `software-null-output/av14k/e0322c2ce0f415da` | 3, 3 |
| 4K AV1 steady RAM | `software-null-output/av14k/e0322c2ce0f415da` | 3, 3 |
| FLAC audio CPU | `software-null-output/flac/4c713fc53815f924` | 3, 3, 3 |
| FLAC audio steady RAM | `software-null-output/flac/4c713fc53815f924` | 3, 3, 3 |
| 2× playback CPU | `software-null-output/speed2x/c45f9405064bf893` | 3, 3, 3 |
| 2× playback steady RAM | `software-null-output/speed2x/c45f9405064bf893` | 3, 3, 3 |
| Peak playback RAM | `software-null-output/avc1080/2d35314f3fa91805` | 3, 3, 3 |

</details>

<details><summary>Runtime outcomes and feature events</summary>

| Player | Runtime outcome | Evidence / limitation |
|---|---|---|
| Foxide | Verified playback 18/21; Verified UI actions 40/41 | adapter-required, sampled-playback-unverified, verified, verified-playback |
| VLC | Verified playback 12/21; Verified UI actions 40/41 | adapter-required, playback-readiness-timeout, sampled-playback-unverified, verified, verified-playback |
| mpv | Verified playback 18/21 | sampled-playback-unverified, verified-playback |
| SMPlayer | Unverified / diagnostic (3 cases); Verified UI actions 20/23 | adapter-required, sampled-playback-unverified, verified |
| FFplay | Unverified / diagnostic (3 cases) | sampled-playback-unverified |
| Kodi | Unverified / diagnostic (3 cases) | early-exit |
| Plex | not-detected | No native executable found; packaged-app/account/Store activation may need a dedicated adapter. |

| Feature | Foxide | VLC | mpv | SMPlayer | FFplay | Kodi | Plex |
|---|---:|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? |
| Audio track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| External subtitle files | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| Variable playback speed | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 |
| Playlists | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | 🥇 |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? | ? |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | ? | ? |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | ? | ? |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? | ? |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? |
| Native picture in picture | ? | ? | ? | ? | ? | ? | ? |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| Persistent playback resume | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | ? | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| Chromecast video output | ? | 🥇 | ? | 🥇 | ? | ? | ? |
| AirPlay video output | ? | ? | ? | ? | ? | ? | ? |
| AirPlay audio receiver | ? | ? | ? | ? | ? | 🥇 | ? |
| Search and download subtitles | ? | ? | ? | 🥇 | ? | ? | ? |
| Video-site URL playback | ? | ? | 🥇 | 🥇 | ? | ? | ? |
| User extension or scripting system | ? | ? | 🥇 | ? | ? | 🥇 | ? |
| Editable keyboard bindings | ? | ? | 🥇 | ? | ? | ? | ? |
| Customizable UI layout or skins | ? | ? | ? | 🥇 | ? | ? | ? |
| Touch or trackpad playback gestures | ? | ? | ? | ? | ? | ? | ? |
| System media-control integration | ? | ? | ? | ? | ? | ? | ? |
| Compact dedicated music view | ? | ? | ? | ? | ? | ? | ? |
| Dark interface theme | ? | ? | ? | ? | ? | ? | ? |
| Music playback artwork view | ? | ? | ? | ? | ? | ? | ? |
| Automatic movie and TV library metadata | ? | ? | ? | ? | ? | 🥇 | ? |
| Browse SMB network media | ? | ? | ? | ? | ? | 🥇 | ? |
| Direct cloud-storage playback | ? | ? | ? | ? | ? | ? | ? |
| Download server media for offline viewing | ? | ? | ? | ? | ? | ? | 🥇 |
| Cross-device playback progress sync | ? | ? | ? | ? | ? | ? | ? |
| Live TV and guide frontend | ? | ? | ? | ? | ? | 🥇 | ? |
| Schedule TV recordings from the player | ? | ? | ? | ? | ? | 🥇 | ? |
| HTTP remote-control interface | ? | ? | ? | ? | ? | 🥇 | ? |
| Documented JSON IPC control | ? | ? | 🥇 | ? | ? | ? | ? |
| HD audio bitstream passthrough | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| 360-degree panoramic video | ? | 🥇 | ? | ? | ? | ? | ? |
| Stereoscopic 3D video | ? | ? | ? | ? | ? | ? | ? |
| Capture playing video to a clip | ? | ? | ? | ? | ? | ? | ? |
| Desktop screen recording | ? | 🥇 | ? | ? | ? | ? | ? |
| Trim and rearrange video clips | ? | ? | ? | ? | ? | ? | ? |
| Audio conversion or transcoding | ? | ? | ? | ? | ? | ? | ? |
| Audio CD ripping | ? | ? | ? | ? | ? | ? | ? |
| Audio or data CD burning | ? | ? | ? | ? | ? | ? | ? |
| Edit media tags | ? | ? | ? | ? | ? | ? | ? |
| Built-in internet-radio directory | ? | ? | ? | ? | ? | ? | ? |
| Video rotation and flip controls | ? | ? | 🥇 | ? | ? | ? | ? |
| ICC display color management | ? | ? | 🥇 | ? | ? | ? | ? |
| User GPU shader pipeline | ? | ? | 🥇 | ? | ? | ? | ? |
| Debanding control | ? | ? | 🥇 | ? | ? | ? | ? |
| Single-frame advance | ? | ? | 🥇 | ? | 🥇 | ? | ? |
| Playlist search and filtering | ? | ? | ? | ? | ? | ? | ? |
| Two simultaneous subtitle tracks | ? | ? | 🥇 | 🥇 | ? | ? | ? |
| Browse UPnP / DLNA media servers | ? | ? | ? | ? | ? | 🥇 | ? |
| Share a library as a UPnP server | ? | ? | ? | ? | ? | 🥇 | ? |
| Game controller UI navigation | ? | ? | ? | ? | ? | 🥇 | ? |
| Browse WebDAV media shares | ? | ? | ? | ? | ? | 🥇 | ? |
| Browse NFS media shares | ? | ? | ? | ? | ? | 🥇 | ? |
| Trakt watched-history scrobbling | ? | ? | ? | ? | ? | ? | ? |
| Native Plex server library integration | ? | ? | ? | ? | ? | ? | 🥇 |
| Native Emby server library integration | ? | ? | ? | ? | ? | ? | ? |
| Native Jellyfin server library integration | ? | ? | ? | ? | ? | ? | ? |
| Skip detected episode intros | ? | ? | ? | ? | ? | ? | 🥇 |
| Music visualization display | ? | 🥇 | ? | ? | ? | 🥇 | ? |

? = undocumented/unverified. N/A = inapplicable media type.

</details>

<!-- foxide-scores:end -->
