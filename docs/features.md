# Foxide feature matrix

All product features below are planned; the current code provides the benchmark/architecture foundation and external-mpv launcher. Capability judgments are hypotheses to verify against pinned competitor versions, rather than claims that every player supports every format/platform.

| Tier | Capability / acceptance example | Baseline emphasis | Planned milestone |
|---|---|---|---|
| 1 | MKV/MP4/WebM/AVI/FLAC plus malformed/unsupported errors; A/B/C/D corpus | VLC/mpv broad compatibility; native OS support varies | v0.1–0.2 |
| 1 | D3D11VA, VideoToolbox, VAAPI/NVDEC where hardware supports codec/profile; visible diagnostics/fallback | Compare actual decoder/GPU path and power, never config alone | v0.1–0.3 |
| 1 | Enumerate/switch audio and subtitle tracks without stale UI/audio loss | VLC/IINA/mpv | v0.2 |
| 1 | External UTF-8 SRT/ASS, font attachments, styling, RTL/shaping and high-DPI placement | VLC/mpv/libass behavior | v0.2 |
| 1 | Exact and keyframe seeks; latest-target scrub coalescing; cancellable thumbnail previews | mpv latency plus IINA-style discoverability | v0.2 |
| 1 | 0.25×–4× speed with supported pitch policy; test sound and sync at extremes | VLC/IINA/mpv | v0.2 |
| 1 | mpv-compatible keyboard defaults, rebinding/conflicts, keyboard focus and accessibility | mpv command vocabulary | v0.2 |
| 1 | File/folder drag/drop, editable ordered playlists and next/previous | VLC/IINA | v0.2 |
| 2 | HDR passthrough where validated, HDR→SDR tone map with reference visual checks | Native HDR quality + mpv/VLC coverage | v0.3–0.4 |
| 2 | Audio delay and subtitle offset, signed values with reset and persistence | VLC/IINA/mpv | v0.3 |
| 2 | Deinterlace and brightness/contrast/gamma; disclose GPU/power impact | VLC/mpv | v0.3 |
| 2 | Chapter names, chapter seek/navigation and keyboard access | VLC/IINA/mpv | v0.3 |
| 2 | PiP and always-on-top, correct platform window semantics | IINA/native UX | v0.4 |
| 2 | HTTP/HLS/RTSP, buffering, reconnect/cancel and credential privacy | VLC/mpv; native support varies | v0.4 |
| 3 | Folder/series auto-queue with natural sorting, episode boundaries and opt-out | Fast series playback workflow | v0.5 |
| 3 | SQLite resume state keyed by file identity; near-end/credits rules and opt-out | Reliable continuation across restarts | v0.5 |
| 3 | Minimal/borderless floating UI, keyboard escape and multi-monitor restoration | Compact viewing | v0.5 |

Maintain a versioned competitor capability sheet during actual testing: mark each cell verified-supported, verified-unsupported, partial or untested with platform/build/file. Do not ship an inferred feature matrix as empirical evidence. Features such as ASS shaping, TrueHD passthrough, HDR and RTSP need specific acceptance assets beyond the synthetic starter suite.
