# Foxide

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/marksmayo/foxide?utm_source=badge)

A Rust desktop media player project, starting with measurable playback and a transparent competitor scoreboard. The current application launches an external mpv window with hardware decoding requested through `hwdec=auto-safe`.

```powershell
.\target\release\foxide.exe "C:\media\movie.mkv"
.\target\release\foxide.exe --speed 1.5 --subtitle "C:\media\movie.srt" "C:\media\movie.mkv"
.\target\release\foxide.exe --autoqueue --resume-dir ".\results\local\resume" "C:\series\episode2.mkv"
```

Build with `cargo build --release --locked`. On Windows, `python scripts/install-mpv.py` installs a verified engine under `tools/mpv`. Playback controls include Space/pause, arrows/seek, Shift+arrows/exact seek, F/fullscreen, J/subtitles, #/audio track, S/screenshot, and Q/quit. Run `foxide --help` for tracks, offsets, tone mapping, picture controls, chapters, URLs, playlists, and window options.

[Performance implementation ledger](docs/performance-review/IMPLEMENTATION.md) · [Capabilities and remaining work](docs/features.md) · [Local players](docs/local-players.md) · [Benchmark protocol](docs/player-games.md) · [Architecture](docs/architecture.md) · [Actions](https://github.com/marksmayo/foxide/actions)

The **Player Games** workflow installs/attempts each platform's native field, records verified playback or labeled frontend diagnostics, updates the GitHub-native graphs, result tables, and expandable feature/metric details below; downloadable HTML remains optional. Feature possession awards shared gold. Performance requires repeated, verified, comparable measurements. Hosted VMs have separate cohorts from physical GPU, battery, and displayed-frame tests.

Development follows red → green tests. CI enforces **90% Rust workspace line coverage** and **90% coverage for medal/report policies**. The custom Rust window/OSD, native PiP, thumbnails, and embedded libmpv GPU integration are still ahead of the current external-engine launcher.

<!-- foxide-scores:start -->
## Player medal scoreboards

Feature support earns **shared gold** for every documented supporter. Unknown is unverified, not unsupported. Counts reflect the documentation reviewed and do not establish an overall best player. Performance medals require at least three verified, comparable repeats on the same platform, host, fixture, scenario, and settings.

[Interactive HTML report](docs/scoreboards/report.html) · [Raw scores](docs/scoreboards/medals.json) · [Feature sources](docs/scoreboards/features.json) · [45 performance protocols](docs/scoreboards/performance-events.json)

### Windows

![Windows feature golds](docs/scoreboards/windows.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 1 | 11 | 2 | Verified playback 18/21 |
| VLC | 20 | 4 | 0 | 7 | Verified playback 15/21 |
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

### macOS

![macOS feature golds](docs/scoreboards/macos.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 2 | 6 | 6 | Verified playback 18/21 |
| VLC | 20 | 9 | 1 | 1 | Verified playback 15/21 |
| mpv | 26 | 3 | 7 | 4 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | not-detected |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Plex | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| foobar2000 | 6 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Infuse | 12 | 0 | 0 | 0 | not-detected |
| IINA | 27 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| QuickTime Player | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

### Linux

![Linux feature golds](docs/scoreboards/linux.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 2 | 11 | 1 | Verified playback 18/21 |
| VLC | 20 | 1 | 0 | 10 | Verified playback 15/21 |
| mpv | 26 | 12 | 2 | 0 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<!-- foxide-scores:end -->
