# Foxide

A Rust desktop media player project, starting with measurable playback and a transparent competitor scoreboard. The current application launches an external mpv window with hardware decoding requested through `hwdec=auto-safe`.

```powershell
.\target\release\foxide.exe "C:\media\movie.mkv"
.\target\release\foxide.exe --speed 1.5 --subtitle "C:\media\movie.srt" "C:\media\movie.mkv"
.\target\release\foxide.exe --autoqueue --resume-dir ".\results\local\resume" "C:\series\episode2.mkv"
```

Build with `cargo build --release --locked`. On Windows, `python scripts/install-mpv.py` installs a verified engine under `tools/mpv`. Playback controls include Space/pause, arrows/seek, Shift+arrows/exact seek, F/fullscreen, J/subtitles, #/audio track, S/screenshot, and Q/quit. Run `foxide --help` for tracks, offsets, tone mapping, picture controls, chapters, URLs, playlists, and window options.

[Capabilities and remaining work](docs/features.md) · [Local players](docs/local-players.md) · [Benchmark protocol](docs/player-games.md) · [Architecture](docs/architecture.md) · [Actions](https://github.com/marksmayo/foxide/actions)

The **Player Games** workflow installs/attempts each platform's native field, records verified playback or labeled frontend diagnostics, generates HTML, and updates the graphs/tables below. Feature possession awards shared gold. Performance requires repeated, verified, comparable measurements. Hosted VMs have separate cohorts from physical GPU, battery, and displayed-frame tests.

Development follows red → green tests. CI enforces **90% Rust workspace line coverage** and **90% coverage for medal/report policies**. The custom Rust window/OSD, native PiP, thumbnails, and embedded libmpv GPU integration are still ahead of the current external-engine launcher.

<!-- foxide-scores:start -->
## Player medal scoreboards

Feature support earns **shared gold** for every documented supporter. Unknown is unverified, not unsupported. Counts reflect the documentation reviewed and do not establish an overall best player. Performance medals require at least three verified, comparable repeats on the same platform, host, fixture, scenario, and settings.

[Interactive HTML report](docs/scoreboards/report.html) · [Raw scores](docs/scoreboards/medals.json) · [Feature sources](benchmarks/features.json) · [45 performance protocols](benchmarks/performance-events.json)

### Windows

![Windows feature golds](docs/scoreboards/windows.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 0 | 2 | 2 | Verified playback 3/3 |
| VLC | 20 | 3 | 0 | 0 | Verified playback 3/3 |
| mpv | 26 | 1 | 2 | 1 | Verified playback 3/3 |
| Windows Media Player | 2 | 0 | 0 | 0 | Not run in this dataset |
| Windows Media Player Legacy | 1 | 0 | 0 | 0 | Not run in this dataset |
| MPC-HC | 10 | 0 | 0 | 0 | Not run in this dataset |
| MPC-BE | 19 | 0 | 0 | 0 | Not run in this dataset |
| PotPlayer | 13 | 0 | 0 | 0 | Not run in this dataset |
| SMPlayer | 9 | 0 | 0 | 0 | Not run in this dataset |
| KMPlayer | 9 | 0 | 0 | 0 | Not run in this dataset |
| GOM Player | 10 | 0 | 0 | 0 | Not run in this dataset |
| FFplay | 4 | 0 | 0 | 0 | Not run in this dataset |
| Kodi | 7 | 0 | 0 | 0 | Not run in this dataset |
| Plex | 4 | 0 | 0 | 0 | Not run in this dataset |
| foobar2000 | 6 | 0 | 0 | 0 | Not run in this dataset |

Measured competitive event cohorts: **4**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

### macOS

![macOS feature golds](docs/scoreboards/macos.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 0 | 0 | 0 | Not run in this dataset |
| VLC | 20 | 0 | 0 | 0 | Not run in this dataset |
| mpv | 26 | 0 | 0 | 0 | Not run in this dataset |
| SMPlayer | 9 | 0 | 0 | 0 | Not run in this dataset |
| FFplay | 4 | 0 | 0 | 0 | Not run in this dataset |
| Kodi | 7 | 0 | 0 | 0 | Not run in this dataset |
| Plex | 4 | 0 | 0 | 0 | Not run in this dataset |
| foobar2000 | 6 | 0 | 0 | 0 | Not run in this dataset |
| Infuse | 12 | 0 | 0 | 0 | Not run in this dataset |
| IINA | 27 | 0 | 0 | 0 | Not run in this dataset |
| QuickTime Player | 4 | 0 | 0 | 0 | Not run in this dataset |

Measured competitive event cohorts: **0**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

### Linux

![Linux feature golds](docs/scoreboards/linux.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 0 | 0 | 0 | Not run in this dataset |
| VLC | 20 | 0 | 0 | 0 | Not run in this dataset |
| mpv | 26 | 0 | 0 | 0 | Not run in this dataset |
| SMPlayer | 9 | 0 | 0 | 0 | Not run in this dataset |
| FFplay | 4 | 0 | 0 | 0 | Not run in this dataset |
| Kodi | 7 | 0 | 0 | 0 | Not run in this dataset |

Measured competitive event cohorts: **0**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<!-- foxide-scores:end -->
