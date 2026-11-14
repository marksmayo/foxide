# Foxide

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/marksmayo/foxide?utm_source=badge)

A Rust desktop media player with a Slint desktop preview and a transparent competitor scoreboard. The new desktop embeds libmpv in Foxide's own window; the existing `foxide` CLI still launches external mpv.

```powershell
cargo build -p player-ui --features desktop --bin foxide-desktop --release --locked
.\target\release\foxide-desktop.exe "C:\media\movie.mkv"
```

The desktop needs a libmpv ABI 2 library: Windows `tools/libmpv/libmpv-2.dll` or `FOXIDE_LIBMPV`, macOS `brew install mpv`, Linux your distribution's `libmpv2`. [Desktop setup and current limits](docs/desktop.md) · [UI action benchmarks](docs/ui-benchmarks.md). The UI includes menus, a playlist, file/URL opening, playback/speed/track controls, external subtitles, volume and fullscreen. Native playback, GPU interop and power acceptance remain in progress.

```powershell
.\target\release\foxide.exe "C:\media\movie.mkv"
.\target\release\foxide.exe --speed 1.5 --subtitle "C:\media\movie.srt" "C:\media\movie.mkv"
.\target\release\foxide.exe --autoqueue --resume-dir ".\results\local\resume" "C:\series\episode2.mkv"
```

Build with `cargo build --release --locked`. On Windows, `python scripts/install-mpv.py` installs a verified engine under `tools/mpv`. Playback controls include Space/pause, arrows/seek, Shift+arrows/exact seek, F/fullscreen, J/subtitles, #/audio track, S/screenshot, and Q/quit. Run `foxide --help` for tracks, offsets, tone mapping, picture controls, chapters, URLs, playlists, and window options.

[Performance implementation ledger](docs/performance-review/IMPLEMENTATION.md) · [Capabilities and remaining work](docs/features.md) · [Local players](docs/local-players.md) · [Benchmark protocol](docs/player-games.md) · [Architecture](docs/architecture.md) · [Actions](https://github.com/marksmayo/foxide/actions)

The **Player Games** workflow installs/attempts each platform's native field, records verified playback or labeled frontend diagnostics, updates the GitHub-native graphs, result tables, and expandable feature/metric details below; downloadable HTML remains optional. Feature possession awards shared gold. Performance requires repeated, verified, comparable measurements. Hosted VMs have separate cohorts from physical GPU, battery, and displayed-frame tests.

Development follows red → green tests. CI enforces **90% Rust workspace line coverage** for default features and **90% coverage for medal/report policies**. Feature-gated desktop/native rendering is not yet covered by that threshold. A separate three-platform desktop workflow builds the GUI and renders a layout preview. Native PiP, thumbnails, drag-and-drop, persistent GUI resume, complete GUI coverage and physical GPU acceptance remain open.

<!-- foxide-scores:start -->
## Player medal scoreboards

Feature support earns **shared gold** for every documented supporter. Unknown is unverified, not unsupported. Counts reflect the documentation reviewed and do not establish an overall best player. Performance medals require at least three verified, comparable repeats on the same platform, host, fixture, scenario, and settings.

Results are readable directly on GitHub below. [Download optional HTML](docs/scoreboards/report.html) to open locally · [Raw scores](docs/scoreboards/medals.json) · [Feature sources](docs/scoreboards/features.json) · [Performance protocols](docs/scoreboards/performance-events.json)

UI events include menus, dialogs, playback controls, scrubbing, fullscreen and resizing. Native UI acknowledgement has a separate cohort from displayed-frame timing. Missing adapters or presentation evidence remain unranked. [UI benchmark protocol](docs/scoreboards/../ui-benchmarks.md).

### Windows

![Windows feature golds](docs/scoreboards/windows.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 2 | 8 | 4 | Verified playback 18/21 |
| VLC | 20 | 3 | 3 | 5 | Verified playback 15/21 |
| mpv | 26 | 10 | 2 | 2 | Verified playback 18/21 |
| Windows Media Player | 2 | 0 | 0 | 0 | installed-adapter-needed |
| Windows Media Player Legacy | 1 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| MPC-HC | 10 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| MPC-BE | 19 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| PotPlayer | 13 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| SMPlayer | 9 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| KMPlayer | 9 | 0 | 0 | 0 | not-detected |
| GOM Player | 10 | 0 | 0 | 0 | installed-adapter-needed |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Plex | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| foobar2000 | 6 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/ef6da1a20405c446 | ms | 🥈 263 | — | 🥇 **116** |
| Paused video CPU / vm-software-null-output/paused/7e08072db2d7aac9 | % of one core | 🥇 **0** | — | 🥇 **0** |
| Paused video steady RAM / vm-software-null-output/paused/7e08072db2d7aac9 | MiB | 🥈 90.3 | — | 🥇 **85.8** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/ef6da1a20405c446 | % of one core | 🥉 35.8 | 🥈 33.9 | 🥇 **33.4** |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/ef6da1a20405c446 | MiB | 🥈 108 | 🥉 139 | 🥇 **103** |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/c0a565d5a5dc739c | % of one core | 🥇 **69.7** | 🥈 180 | 🥉 193 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/c0a565d5a5dc739c | MiB | 🥉 416 | 🥈 412 | 🥇 **362** |
| 4K AV1 CPU / vm-software-null-output/av14k/9275ddbae5df14c7 | % of one core | 🥈 167 | 🥇 **165** | 🥉 170 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/9275ddbae5df14c7 | MiB | 🥈 480 | 🥉 555 | 🥇 **473** |
| FLAC audio CPU / vm-software-null-output/flac/a5e820a71e67aaf1 | % of one core | 🥈 1.85 | 🥉 2.02 | 🥇 **1.32** |
| FLAC audio steady RAM / vm-software-null-output/flac/a5e820a71e67aaf1 | MiB | 🥉 54.9 | 🥇 **25.1** | 🥈 50.5 |
| 2Ã— playback CPU / vm-software-null-output/speed2x/e13283e5559ba6ab | % of one core | 🥉 72.4 | 🥇 **69.4** | 🥈 72.2 |
| 2Ã— playback steady RAM / vm-software-null-output/speed2x/e13283e5559ba6ab | MiB | 🥈 110 | 🥉 140 | 🥇 **105** |
| Peak playback RAM / vm-software-null-output/avc1080/ef6da1a20405c446 | MiB | 🥈 108 | 🥉 140 | 🥇 **104** |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

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
| Aâ€“B repeat | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | N/A |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

### macOS

![macOS feature golds](docs/scoreboards/macos.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 2 | 8 | 4 | Verified playback 18/21 |
| VLC | 20 | 9 | 1 | 1 | Verified playback 15/21 |
| mpv | 26 | 3 | 5 | 6 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | not-detected |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Plex | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| foobar2000 | 6 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Infuse | 12 | 0 | 0 | 0 | not-detected |
| IINA | 27 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| QuickTime Player | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/c474b6ca2dbd3017 | ms | 🥈 278 | — | 🥇 **138** |
| Paused video CPU / vm-software-null-output/paused/64ee3312cfd8e974 | % of one core | 🥈 0.914 | — | 🥇 **0.851** |
| Paused video steady RAM / vm-software-null-output/paused/64ee3312cfd8e974 | MiB | 🥈 167 | — | 🥇 **163** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/c474b6ca2dbd3017 | % of one core | 🥈 28.9 | 🥇 **9.24** | 🥉 51.8 |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/c474b6ca2dbd3017 | MiB | 🥉 223 | 🥇 **94.5** | 🥈 221 |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/3a8e968588e914cb | % of one core | 🥉 192 | 🥇 **4.74** | 🥈 190 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/3a8e968588e914cb | MiB | 🥉 526 | 🥇 **122** | 🥈 518 |
| 4K AV1 CPU / vm-software-null-output/av14k/b3b2e17733c86e02 | % of one core | 🥇 **112** | 🥈 115 | 🥉 122 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/b3b2e17733c86e02 | MiB | 🥇 **624** | 🥉 688 | 🥈 628 |
| FLAC audio CPU / vm-software-null-output/flac/2311e732ede614f0 | % of one core | 🥈 2.52 | 🥇 **2.46** | 🥉 5.97 |
| FLAC audio steady RAM / vm-software-null-output/flac/2311e732ede614f0 | MiB | 🥈 171 | 🥇 **67.3** | 🥉 171 |
| 2Ã— playback CPU / vm-software-null-output/speed2x/faef3ce473944833 | % of one core | 🥈 74.2 | 🥇 **17.7** | 🥉 79.7 |
| 2Ã— playback steady RAM / vm-software-null-output/speed2x/faef3ce473944833 | MiB | 🥉 222 | 🥇 **98** | 🥈 219 |
| Peak playback RAM / vm-software-null-output/avc1080/c474b6ca2dbd3017 | MiB | 🥈 227 | 🥇 **95.2** | 🥉 234 |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

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
| Aâ€“B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | 🥇 |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

### Linux

![Linux feature golds](docs/scoreboards/linux.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 3 | 10 | 1 | Verified playback 18/21 |
| VLC | 20 | 3 | 1 | 7 | Verified playback 15/21 |
| mpv | 26 | 9 | 2 | 3 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/7652926fd1513544 | ms | 🥇 **155** | — | 🥈 157 |
| Paused video CPU / vm-software-null-output/paused/27f5fa6a1d32902a | % of one core | 🥇 **0** | — | 🥇 **0** |
| Paused video steady RAM / vm-software-null-output/paused/27f5fa6a1d32902a | MiB | 🥈 103 | — | 🥇 **100** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/7652926fd1513544 | % of one core | 🥈 35 | 🥉 35 | 🥇 **34.9** |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/7652926fd1513544 | MiB | 🥈 120 | 🥉 159 | 🥇 **117** |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/e7713a4c6656e4fd | % of one core | 🥈 192 | 🥇 **182** | 🥉 192 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/e7713a4c6656e4fd | MiB | 🥈 389 | 🥉 469 | 🥇 **387** |
| 4K AV1 CPU / vm-software-null-output/av14k/28979e64eb3a89a1 | % of one core | 🥇 **177** | 🥈 180 | 🥉 180 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/28979e64eb3a89a1 | MiB | 🥈 486 | 🥉 552 | 🥇 **484** |
| FLAC audio CPU / vm-software-null-output/flac/b88972b541f16f49 | % of one core | 🥈 0.673 | 🥉 1.14 | 🥇 **0.505** |
| FLAC audio steady RAM / vm-software-null-output/flac/b88972b541f16f49 | MiB | 🥉 67.9 | 🥇 **38.1** | 🥈 65.3 |
| 2Ã— playback CPU / vm-software-null-output/speed2x/24f5949aaa56bd48 | % of one core | 🥈 71.1 | 🥇 **68.6** | 🥉 71.4 |
| 2Ã— playback steady RAM / vm-software-null-output/speed2x/24f5949aaa56bd48 | MiB | 🥈 121 | 🥉 160 | 🥇 **118** |
| Peak playback RAM / vm-software-null-output/avc1080/7652926fd1513544 | MiB | 🥈 120 | 🥉 159 | 🥇 **118** |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

| Feature | Foxide | VLC | mpv | SMPlayer | FFplay | Kodi |
|---|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 |
| Audio track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| External subtitle files | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Variable playback speed | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? |
| Playlists | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | ? |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | ? |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 |
| Native picture in picture | ? | ? | ? | ? | ? | ? |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Persistent playback resume | 🥇 | ? | 🥇 | 🥇 | ? | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Aâ€“B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

<!-- foxide-scores:end -->
