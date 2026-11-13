# Foxide competitor scoreboard

Session ID: ___ · Host/OS/GPU/driver: ___ · Display/HDR/brightness: ___ · Power plan/source: ___

Media manifest SHA-256: ___ · Tool/executable manifest SHA-256: ___ · Capture tools/versions: ___

Use one row per **player/build/profile/file/cache condition**. Keep OS results separate. Every value needs a raw-artifact link and measurement method. `N/A (reason)` means unsupported or unavailable; it never means zero. TTFF applies to video; D uses a separately measured audio-onset metric.

| Player/build | File/profile/cache | Trials | Display TTFF p50/p95 ms | Internal restart proxy p50/p95 ms | Keyframe seek p50/p95 ms | Exact seek p50/p95 ms | Steady/peak RSS MB | Dedicated/shared GPU MB | Package idle/play W | System/battery play W | CPU % one-core | GPU engines % | Decoder/output drops | Display interval σ/p99 ms | Installed MB | Evidence/status |
|---|---|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| VLC / ___ | A / defaults / warm | ___ | ___ | N/A | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| mpv / ___ | A / defaults / warm | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| mpv / ___ | A / auto-safe / warm | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| Windows Media Player Modern / ___ | A / defaults / warm | ___ | ___ | N/A | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| IINA / ___ | A / defaults / warm | ___ | ___ | N/A | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| QuickTime / ___ | A / defaults / warm | ___ | ___ | N/A | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | untested |
| Foxide / ___ | A / battery / warm | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | ___ | unimplemented |

Repeat for B/C/D, cold-reboot, subtitles on/off and the agreed presentation/quality profiles. Record GPU transfer path, actual hardware decoder, seek landing error, missing/failed trial count and power confidence intervals in the row's evidence. Targets are in decimal MB; automatic process reports use MiB. Archive process samples, events, trace/capture, package file list and analysis code.
