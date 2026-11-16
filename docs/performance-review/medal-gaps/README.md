# Medal gap audit

Source: main b6b7f594e92195c474113582eb7b0381f9a2c442. All 80 feature events and all 393 performance definitions were reviewed; 26 measured event types contain a non-gold Foxide result. The [machine-readable audit](audit.json) retains every event, field, value, disposition and candidate mapping. No change can be promised literally risk-free.

## Ranked performance changes

| Rank | Change | Expected saving and verification |
|---|---|---|
| 1 | unix-launcher-handoff (implemented) | Removes the resident launcher address space and one separate process spawn on Unix. Windows remains unchanged. Measure exact RAM/time on matched hosted runners. PID identity regression failed on GitHub before exec and passed after handoff; original error/argument tests retained. |
| 2 | shared-track-snapshots (implemented) | Replaces O(track-count) vector/string copies in each worker/public/UI snapshot copy with constant-size Arc reference updates. No percentage or watts claim. Snapshot clone initially had a different track-array pointer; after the change cloned immutable metadata shares storage. Copy-on-write selection tests remain valid. |
| 3 | unchanged-status-refresh (implemented) | Avoids up to five status-string formats and UI status writes per second during steady playback. Actual CPU impact needs measurement. Position-tick test failed by overwriting a command error, then passed. New-file transition regression separately failed and passed. |
| 4 | lazy-idle-engine (deferred-medium-risk) | Potentially avoids loading libmpv and creating renderer resources for empty-window UI work. Not measured. Requires changing rendering-context initialization and first-load lifecycle; cannot call zero risk without GPU lifecycle tests. |
| 5 | idle-texture-allocation (deferred-medium-risk) | Potentially defer an unused idle render target. Not measured. Requires a display-backed no-frame/first-frame repro and resize/teardown tests before changing renderer allocation. |

The ranking is an engineering estimate of removed work, not measured CPU percentages. Hosted software/null-output results use the external-mpv CLI; they do not certify desktop GPU efficiency. Existing native-window measurements are older local data. There is no safe blanket codec/decoder setting that guarantees gold on every machine. Quality, fixture complexity and eligibility rules remain intact.

## Feature gaps

Seven missing features are implemented, and the existing dark theme is verified separately. The other 49 need broader integrations or acceptance resources. They remain unimplemented and unranked, rather than being marked supported because libmpv might expose an option.

| Event | Disposition | Evidence or remaining work |
|---|---|---|
| Seekbar thumbnail previews | requires-broader-integration-and-acceptance | Needs a bounded thumbnail decoder/cache and scrubbing presentation tests; not a decoder-readback shortcut. |
| Native picture in picture | requires-broader-integration-and-acceptance | Native picture-in-picture needs OS-specific window lifecycle, focus and multi-display acceptance. |
| Audio equalizer / DSP | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| ReplayGain | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| Gapless audio playback | requires-broader-integration-and-acceptance | Needs queued/preloaded tracks and audio-loopback evidence of gap-free transitions. |
| DVD / Blu-ray structure playback | requires-broader-integration-and-acceptance | Needs disc fixtures, optional navigation libraries and packaging/licensing review. |
| Chromecast video output | requires-broader-integration-and-acceptance | Needs device discovery, receiver sessions and a real cast destination for acceptance. |
| AirPlay video output | requires-broader-integration-and-acceptance | Needs a platform sender integration and physical receiver validation. |
| AirPlay audio receiver | requires-broader-integration-and-acceptance | Needs receiver networking, protocol compatibility and audio output validation. |
| Search and download subtitles | requires-broader-integration-and-acceptance | Needs provider credentials, account/rate-limit handling and subtitle download validation. |
| Video-site URL playback | requires-broader-integration-and-acceptance | Needs explicit supported yt-dlp integration, dependency discovery and network acceptance. |
| User extension or scripting system | requires-broader-integration-and-acceptance | Executes third-party code; needs a trust, permission and lifecycle design. |
| Editable keyboard bindings | requires-broader-integration-and-acceptance | Needs persisted user settings and cross-platform key/modifier/focus handling. |
| Customizable UI layout or skins | requires-broader-integration-and-acceptance | Needs a layout/theme configuration model and accessibility/layout acceptance. |
| Touch or trackpad playback gestures | requires-broader-integration-and-acceptance | Needs touch/trackpad input adapters and real device gesture validation. |
| System media-control integration | requires-broader-integration-and-acceptance | Needs separate Windows/macOS/Linux session integrations and ownership handling. |
| Compact dedicated music view | requires-broader-integration-and-acceptance | Needs a separate compact layout, resize/focus restoration and audio/video behavior tests. |
| Dark interface theme | already-implemented-claim-audit | Existing dark desktop palette; preview verification precedes claim correction. |
| Music playback artwork view | requires-broader-integration-and-acceptance | May already be available through the engine, but requires an embedded-art audio fixture and composited-image proof. |
| Automatic movie and TV library metadata | requires-broader-integration-and-acceptance | Needs a metadata provider, API credentials, cache/privacy policy and library indexing. |
| Browse SMB network media | requires-broader-integration-and-acceptance | Direct URL playback does not implement share browsing or credential storage. |
| Direct cloud-storage playback | requires-broader-integration-and-acceptance | Needs OAuth/provider APIs and protected credential storage. |
| Download server media for offline viewing | requires-broader-integration-and-acceptance | Needs authenticated server clients, storage quotas and cancellation/integrity handling. |
| Cross-device playback progress sync | requires-broader-integration-and-acceptance | Needs an authenticated sync service, conflict resolution and privacy controls. |
| Live TV and guide frontend | requires-broader-integration-and-acceptance | Needs tuner/PVR provider integration and channel/guide fixtures. |
| Schedule TV recordings from the player | requires-broader-integration-and-acceptance | Needs scheduling, backend ownership and safe recording-file lifecycle. |
| HTTP remote-control interface | requires-broader-integration-and-acceptance | Needs authentication, network binding and an explicit command-authorization policy. |
| Documented JSON IPC control | requires-broader-integration-and-acceptance | Needs a documented authenticated/permissioned local control endpoint; exposing arbitrary mpv commands is not a safe default. |
| HD audio bitstream passthrough | requires-broader-integration-and-acceptance | Needs a compatible physical receiver and audio route; cannot be certified by hosted null output. |
| 360-degree panoramic video | requires-broader-integration-and-acceptance | Needs projection controls and visual/presentation fixtures. |
| Stereoscopic 3D video | requires-broader-integration-and-acceptance | Needs stereoscopic output controls and display-format acceptance. |
| Capture playing video to a clip | requires-broader-integration-and-acceptance | Needs an explicit destination/overwrite policy and recording lifecycle. |
| Desktop screen recording | requires-broader-integration-and-acceptance | Needs OS capture permissions and capture-source selection. |
| Trim and rearrange video clips | requires-broader-integration-and-acceptance | Needs an editing/export pipeline with synchronization and output integrity tests. |
| Audio conversion or transcoding | requires-broader-integration-and-acceptance | Needs an export codec/profile API and safe output-file handling. |
| Audio CD ripping | requires-broader-integration-and-acceptance | Needs drive discovery, extraction/error handling and physical media. |
| Audio or data CD burning | requires-broader-integration-and-acceptance | Needs writing APIs and physical media; destructive recording is not a zero-risk change. |
| Edit media tags | requires-broader-integration-and-acceptance | Writes source media; requires explicit save semantics and format-specific integrity tests. |
| Built-in internet-radio directory | requires-broader-integration-and-acceptance | Needs a sourced provider/catalog, network failure handling and provider-term review. |
| Video rotation and flip controls | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| ICC display color management | requires-broader-integration-and-acceptance | Needs profile loading, display switching and color-accuracy fixtures on each renderer. |
| User GPU shader pipeline | requires-broader-integration-and-acceptance | Needs explicit file loading, shader error handling and GPU acceptance; arbitrary GPU code is not zero risk. |
| Debanding control | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| Single-frame advance | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| Playlist search and filtering | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| Two simultaneous subtitle tracks | implemented-with-red-green-tests | Desktop control or playlist filter implemented; see docs/medal-features.md. |
| Browse UPnP / DLNA media servers | requires-broader-integration-and-acceptance | Needs discovery/browsing adapters, XML validation and isolated server fixtures. |
| Share a library as a UPnP server | requires-broader-integration-and-acceptance | Introduces an inbound network service; needs permissions, discovery and access controls. |
| Game controller UI navigation | requires-broader-integration-and-acceptance | Needs controller mapping, hot-plug, focus and device acceptance. |
| Browse WebDAV media shares | requires-broader-integration-and-acceptance | Needs a credentialed WebDAV client, safe XML/path handling and server fixtures. |
| Browse NFS media shares | requires-broader-integration-and-acceptance | Needs network-share discovery/mount integration and platform permission handling. |
| Trakt watched-history scrobbling | requires-broader-integration-and-acceptance | Needs user authentication, sync conflict handling and rate limits. |
| Native Plex server library integration | requires-broader-integration-and-acceptance | Needs authenticated library/session integration, not direct URL playback. |
| Native Emby server library integration | requires-broader-integration-and-acceptance | Needs authenticated library/session integration, not direct URL playback. |
| Native Jellyfin server library integration | requires-broader-integration-and-acceptance | Needs authenticated library/session integration, not direct URL playback. |
| Skip detected episode intros | requires-broader-integration-and-acceptance | Needs marker detection/provider integration and seek-boundary acceptance. |
| Music visualization display | requires-broader-integration-and-acceptance | Needs an audio-analysis/render pipeline and resource/frame-pacing benchmarks. |

## Red/green evidence

- Unix process identity failed on the GitHub runner: engine PID 2934 versus launcher PID 2933. The green handoff retains one PID and the engine exit code. Windows continues waiting for the child process.
- Each of six control families and playlist filtering had a failing public dispatch test before implementation.
- Snapshot sharing failed on distinct cloned track-array addresses, then passed with immutable shared storage.
- UI status preservation failed on a position tick; the new-file transition then exposed and fixed a missing invalidation condition.
- Universal-feature exclusion failed before the rule, then passed without mutating the source catalog.
- Medal-first layout failed because later platform podium tables followed the first chart, then passed after regrouping all tables above charts.
- Native acceptance uses the same public dispatch interface for 18 real engine commands. It observes asynchronous properties before proceeding and retains primary subtitle selection.

## Report rules

Medal tables (overall, performance, feature, per-platform, per-field and feature matrices) appear before every chart. Measurement and runtime details follow the charts. A feature contest is omitted on a platform when at least two eligible entrants all have documented gold; inapplicable media types do not block exclusion, but unknown/planned/invalid claims do. Source evidence is preserved in features.json and omitted_universal_features in medals.json, and omitted events do not inflate scores. Single-entrant events are retained rather than being treated as a competitive universal result.
