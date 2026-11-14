# Foxide capabilities and feature events

The `foxide` CLI launches external mpv. The optional `foxide-desktop` preview now embeds libmpv in a Slint window, with menus, a playlist, playback/speed/volume/track controls, external subtitles, fullscreen and file/URL opening. See [desktop setup and limits](desktop.md). The table below describes the established CLI interface; GUI playback and GPU acceptance remain separate.

## Available controls

| Capability | Current interface | Verification |
|---|---|---|
| Local containers and codecs | One or more file arguments; decoding supplied by mpv/FFmpeg | Windows H.264 playback observed with D3D11VA; codec corpus in Actions |
| Hardware decoding | Requests `hwdec=auto-safe` | Activation does not certify zero-copy interop or power parity |
| Audio/subtitle tracks | `--audio-track ID`, `--subtitle-track ID` | Public playback policy and launcher integration tests |
| External SRT/ASS | Repeatable `--subtitle FILE`; rendered by mpv/libass | Path and argument tests; visual shaping/font acceptance remains separate |
| Playback speed | `--speed 0.25` through `--speed 4` | Finite/range validation before launch |
| Ordered playlists | Multiple paths, a playlist file, or a folder | Natural ordering and non-media exclusion tests |
| Folder autoqueue | `--autoqueue episode2.mkv` | Starts at the selected file in the naturally sorted direct folder |
| Exact seeks / keyboard controls | mpv window shortcuts; Shift+arrows for exact seeks | Engine-provided; future Rust OSD policy has shortcut tests |
| Video adjustments | `--brightness`, `--contrast`, `--gamma` (-100..100), `--deinterlace` | Validation and engine-option tests |
| HDR input / SDR tone mapping | mpv HDR handling; `--tone-map` targets BT.709 / BT.1886 | Configuration tested; display passthrough/visual reference checks pending |
| Signed synchronization offsets | `--audio-delay`, `--subtitle-delay` in seconds | Non-finite values rejected; signs preserved |
| Chapters | `--chapter INDEX` (zero-based); mpv chapter keys | Cannot combine chapter start with `--start` |
| Network playback | HTTP(S), HLS URLs, RTSP URLs | Individual arguments; live/network resilience acceptance pending |
| Persistent resume | `--resume-dir DIRECTORY` | mpv watch-later state; saves on graceful exit. SQLite history is a later milestone |
| Borderless floating window | `--minimal --on-top` | mpv borderless/always-on-top settings |
| Screenshots | mpv screenshot key | Engine-provided |
| A–B repeat | `--ab-start A --ab-end B` | Finite `0 <= A < B` validation |
| Repeat playlist | `--loop-playlist` | Explicit loop policy |
| Benchmark diagnostics | `--headless`, `--paused`, `--benchmark-script FILE`, `--print-command` | Null-output runs have separate cohorts |

`--print-command` outputs a JSON argument vector without starting playback. Media/subtitle paths and values are passed directly without shell evaluation.

## Implementation backlog

The [30-event feature catalog](../benchmarks/features.json) also tracks seekbar thumbnails, native PiP, an audio equalizer/DSP interface, ReplayGain controls, gapless-output acceptance, optical-disc workflows, and casting. These earn no Foxide gold until exposed and documented to the same event definition.

The original brief additionally requires a custom GPU OSD, editable drag/drop playlists, cancellable thumbnails, reconnect/buffering controls, a SQLite/redb library, accessibility, and native OS window behavior. Delegated mpv controls are an intermediate release, not completion of the native Rust architecture.

Each slice follows red → green tests through public interfaces. See [playback policy](../crates/player-core/src/playback.rs), [folder queue](../crates/player-core/src/queue.rs), [launcher](../crates/player-cli/src/main.rs), [policy tests](../crates/player-core/tests/playback.rs), and [launcher tests](../crates/player-cli/tests/launch.rs).
