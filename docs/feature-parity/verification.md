# Customization acceptance

Implementation source: f957f4da08af3a4a1171f3ac5f5475068ba6d887.

[Three-platform desktop acceptance](https://github.com/marksmayo/foxide/actions/runs/43972156451), [platform checks and coverage gates](https://github.com/marksmayo/foxide/actions/runs/43972156649), [native playback controls](https://github.com/marksmayo/foxide/actions/runs/43972156518), [compositor regression](https://github.com/marksmayo/foxide/actions/runs/43972156442), Windows packaging and [CodSpeed](https://github.com/marksmayo/foxide/actions/runs/43972156363) pass for the implementation source.

| Coverage scope | Measured line coverage |
|---|---:|
| Default Rust workspace | 95.44% |
| New settings module | 94.19% |
| Full feature-gated desktop/GPU Rust | 82.83% |

The default and new-module scopes exceed 90%. Full desktop/GPU coverage remains below the 90% target; no badge or claim treats default coverage as full application coverage. All 111 Python tests pass after refreshing the three feature claims.

CodSpeed reports 263 existing cases unchanged and 16 new cases, with no regressions. Five existing optimized-out cases and three syscall-dominated cases retain measurement warnings. The 279 cases run across six targets. This is CPU policy/settings evidence, not a displayed-frame, startup, GPU-power or battery measurement.

Intermediate combined-crate builds regressed existing tiny state benchmarks despite unchanged policy source. Explicit inlining and duration rewrites did not fully solve that; those experimental policy edits were reverted. Settings parsing/storage and its 16 benchmarks now live in the separate player-settings crate/target. The original playback-policy library and existing action benchmark source are byte-for-byte unchanged from main; the full hosted comparison is green with every existing case retained and thresholds unchanged. This result is consistent with code-generation coupling, rather than evidence of a player-wide speed gain.

Three feature events are accepted: editable keys, customizable light/dark skins with an editable accent, and compact music layout. Counts after scoring are 34 Windows / 33 macOS / 32 Linux feature golds. The different totals reflect platform eligibility and omitted universal events; do not add them across platforms. All 46 other baseline feature gaps remain queued in remaining.json.

The README retains previously collected playback measurements and their original source/run identities. This feature-score refresh does not represent a new full playback benchmark run. The raw measurement bundles remain byte-for-byte unchanged. Medal tables and feature matrices remain above all charts, and universal gold events remain excluded.

## Audit checkpoint a31fb55

[Hosted desktop/GPU coverage](https://github.com/marksmayo/foxide/actions/runs/44007076549) measured **90.605%** (3,115/3,438 lines), including real desktop startup/shutdown, normal/compact previews and compositor tests. The 90% aggregate gate passed without source exclusions. CodSpeed passed at this checkpoint after the single-pass statistics change. The later video-site implementation requires its own new verification; these numbers are not a claim for later commits. The Windows loopback HTTP fixture subsequently received an explicit blocking-mode/partial-read correction.

## Video-site protocol checkpoint 4c3dd798

[Native helper acceptance](https://github.com/marksmayo/foxide/actions/runs/44011195496) passed on Windows, macOS and Linux: an explicitly selected helper with spaces/commas in its filename returned separate video/audio streams; mpv forwarded the HTTP headers and initialized both decoders through an internal restart. This is deterministic protocol acceptance, not a claim of live-site availability. The compositor job hit an upstream FFmpeg-header incompatibility after ubuntu-latest advanced; the pinned mpv regression build now uses ubuntu-24.04, matching the independent control job.

## Artwork compositor checkpoint 5b9f90a

The [hosted compositor run](https://github.com/marksmayo/foxide/actions/runs/44017373925) passed all twelve uninstrumented trials and the instrumented artwork/motion compositor checks. The actual decoded cover appears upright after Slint composition:

![Actual hosted shared-texture artwork output](images/artwork-native.png)

The later null-output metadata coverage pass exposed an unsupported new V_FFV1 tag in its fixture on the older distribution engine. The fixture now uses lossless H.264; this checkpoint is not a claim that the aggregate coverage job passed. Shader inversion also produced the expected cyan/yellow output before the deliberate compile-error test exposed a missing render-API warning prefix, corrected in 0d936a03. Both features remain unawarded until their current acceptance is green.

## Accepted native/services checkpoint 8e1abde

[All-platform live provider acceptance](https://github.com/marksmayo/foxide/actions/runs/44062762763), public desktop callbacks, native metadata/protocol tests, packaging, CodSpeed and the [actual compositor/coverage run](https://github.com/marksmayo/foxide/actions/runs/44062770533) pass. The latter measured **90.85%** (3,872/4,262 lines). Real music audio advances its clock, selected decoder properties identify actual decoders, and page identity survives extraction. Twelve trials verify shared-texture artwork, shader inversion, compile-error reporting and restored pixels.

Internet-radio directory, video-site URL playback with an explicit helper, embedded artwork and user GPU shaders are now accepted: **7 of 49 original gaps closed; 42 remain**. Exact per-platform evidence is in acceptance.json. Null-output live checks and software-GL compositor proofs do not certify native GPU power, physical audio routes or displayed TTFF. Existing competitive performance measurements were preserved byte-for-byte during the feature-score refresh.

## Manual ICC checkpoint c824368

All checks passed, including CodSpeed, three-platform UI/native profile application/reset, twelve compositor trials and the 90% aggregate gate. Actual pixels changed RGB **128 → 55 → 128**, matching the independent Little CMS fixture. Full desktop/GPU line coverage: **90.59%**, 4,024/4,442 lines. This accepts manual RGB display-profile selection for the session; automatic per-monitor profile discovery/switching and physical calibration remain outside the claim. **8 original gaps accepted; 41 remain.** Exact immutable jobs are recorded in acceptance.json.

## Chromecast physical acceptance checklist

The desktop Cast panel, bounded discovery, explicit certificate review, remembered receiver settings and playback worker have deterministic UI/service coverage. Physical receiver confirmation currently covers silent H.264 video and pause/seek/resume/stop for a 30-second window on two receivers. It does not yet establish physical desktop-panel acceptance, audible audio, or codecs beyond H.264. No Chromecast feature medal is claimed.

For the remaining owner-device check, use a local test file and report each item as pass, fail or blocked:

| Check | Result |
|---|---|
| Discover a local receiver in the Cast panel | Pending |
| Review its certificate pin and explicitly remember it | Pending |
| Close and reopen the panel; confirm the remembered receiver appears | Pending |
| Forget the receiver and confirm it is removed | Pending |
| Start local video and verify moving playback; pause, seek, resume and stop | Pending |
| Start local audio and verify it is audible; then stop it | Pending |

Report behavior only. Do not include receiver names, addresses, certificate fingerprints, or private media paths in the PR or repository. Update this checklist and `remaining.json` only after the results are reported; a failure should be recorded as a concrete follow-up rather than treated as acceptance.

## System media controls implementation checkpoint

The desktop adapter connects playwire to the existing player action/state path for Windows SMTC, macOS Now Playing, and Linux MPRIS. It attaches once the desktop engine is ready, including before a track loads, and rate-limits retries while the native window handle is not ready. It publishes title, artist, album, playback state, seek position and supported transport capabilities; OS callbacks are queued back to the Slint event loop. Local file paths and artwork URLs are withheld from system metadata. Unit tests cover idle attachment retry policy, command routing, idempotent play/pause state, metadata, stop clearing, seek limits and per-track identity. A native libmpv acceptance test and tagged fixture are wired into the Linux native-controls workflow. Local Windows desktop/UI tests and strict UI clippy pass; hosted all-platform session and coverage evidence is pending. This remains in progress until hosted OS checks and user-facing interaction are verified.

## Touch and trackpad gesture implementation checkpoint

The desktop video surface maps horizontal trackpad scroll and horizontal pointer/touch drags to relative seeking, and vertical scroll/drags to volume. The software-window UI test checks the 31px/32px gesture threshold, seek direction and ±15-second clamps, volume limits, scroll behavior, and disabled playback state. This remains in progress until physical trackpad and touchscreen behavior is verified; no feature medal is claimed.

## Game controller navigation implementation checkpoint

The desktop now polls standardized gamepad events and routes D-pad focus traversal, confirm/back, playback toggle and previous/next through Slint or the shared playback action path. It surfaces controller connection and disconnection events without publishing device names. Mapping tests cover supported and ignored controls. Linux CI explicitly installs libudev development headers for the gilrs backend. This remains in progress until physical hot-plug and focus-order behavior is verified on supported desktop platforms.

## Stereoscopic 3D implementation checkpoint

The Video menu exposes supported side-by-side and over/under source layouts with red/cyan, over/under or side-by-side output. A dedicated labeled FFmpeg `stereo3d` filter is replaced when another mode is chosen and removed by “Off”, leaving unrelated video filters intact. State tests verify every exposed mapping and reject invalid mode values. FFmpeg smoke checks process each selected graph. Physical display/glasses acceptance with an actual stereoscopic sample remains pending; no feature medal is claimed.

