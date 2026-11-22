# Foxide phased implementation roadmap

Each milestone exits on working software and evidence. Absolute latency/RAM/size targets remain hypotheses until the first platform baselines exist.

| Milestone | Deliverable | Exit gate |
|---|---|---|
| Foundation (current 0.0.1) | Cargo workspace; process benchmark runner; synthetic media generator/probes/hashes; comparison template; initial ADRs and protocols | Compile/lint/tests, Windows sampler and VLC smoke trial; clearly disclose unimplemented collectors/native adapters |
| Benchmark completion | Content-aware display/pacing collectors; GPU/VRAM and calibrated power imports; native app activation/seek adapters; automated environment/provenance manifests; matched-run regression checker | Replay fixtures validate known timing boundaries; metric coverage enforced; cross-player startup/seek comparability; cold reboot orchestration and real 10-minute baselines on each physical platform |
| v0.1 | Audited dynamic libmpv client wrapper; headless control/audio mode plus minimal windowed video; `hwdec=auto-safe`; lifetime-safe render loop; no CPU readback on validated paths | A/B/C/D playback, first-frame/seek timestamps, open/close/resize stress, actual decoder/interops captured. GL portable spike and supported Windows D3D11 spike evaluated; audio clock unaffected by stalled UI |
| v0.2 | egui OSD, file/drop/playlist, track and external subtitle selection, exact/keyframe scrub and thumbnail worker, speed controls, shortcuts | Every Tier 1 row has automated/instrumented or documented visual acceptance; thumbnail cancellation, bounded memory and accessibility/IME review; no large power regression with hidden controls |
| v0.3 | Platform efficiency: Windows adapter/device tuning, macOS native-path decision, Linux driver/session coverage; delay/sync, chapters, filters | Trace evidence of no CPU transfer on supported GPU paths; missing hardware decode reported; native-quality color/subtitles; ten-minute power confidence interval meets chosen parity tolerance |
| v0.4 | HDR passthrough/tone mapping, PiP, HTTP/HLS/RTSP and network recovery | Authored HDR visual reference checks; sleep/wake/device-loss recovery; buffered stream cancellation; corrupted/untrusted media robustness; Tier 2 complete across supported platform matrix |
| v0.5 | Natural folder auto-queue, SQLite resume, borderless/minimal UI; narrow release dependency builds | Tier 3 accepted; resume/move/replacement semantics; idle power and startup remain within approved budgets; complete dependency/installed-size inventory |
| v0.9 | Release candidate, signed/notarized packages and repeatable native libraries; update strategy; long playback soak | License/source/notice audit, provenance/SBOM, platform packaging and uninstall checks; memory/resource leak checks; complete dedicated-machine benchmark report for all supported builds |
| v1.0 | Feature-complete supported cross-platform release with published measurement protocol/results | All three tiers accepted; complete metric evidence, no approved-baseline regressions, known hardware/codec limitations documented; measured target misses require a recorded product decision rather than hidden exclusions |

## Release and update rollout

| Platform | Current state | Remaining work | Acceptance |
|---|---|---|---|
| Windows | Per-user installer and ZIP are available. The updater checks published releases, verifies SHA-256, downloads automatically, and stages the update for user approval before restart. Preview `v0.0.1-preview.2` is published. | Test the update from the previous preview install and fix any issues reported. | The existing preview install detects the newer preview, verifies and stages the ZIP, preserves user data, and updates only after approval. |
| macOS | Release packaging and automatic updates are not implemented. | Add a repeatable signed/notarized release, update notifications, verified downloads, staging, and approval before installation/restart. | Install and update from a previous version on supported macOS; verify signature, checksum, preserved user data, and rollback/failure behavior. |
| Linux | Release packaging and automatic updates are not implemented. | Define supported package formats and update channels; add release packaging, notifications, verified downloads, staging, and approval before installation/restart. | Install and update from a previous version on the supported Linux matrix; verify package integrity, preserved user data, and failure recovery. |

Planning estimate for the macOS and Linux release/update work together: **15–30 engineering days**. Signing credentials, notarization, and distribution coverage can add calendar time. This is a rough estimate, not a commitment.

Implementation order within v0.1: pin/audit native dependencies → wrap client lifecycle/options/errors → event worker and bounded commands → audio-only smoke → own window/context → render API initialization before file load → GPU wake/render/swap → same-surface OSD spike → fallback diagnostics → external presentation timing → power/GPU acceptance. Do not build a rich library UI before the surface-sharing and audio-clock constraints are proven.

Revisit ADRs after the first native benchmarks, after each platform spike and before packaging. If libmpv's public render APIs or the GL path cannot meet native power, compare a GStreamer/native presentation spike under the same harness, then consider a custom FFmpeg pipeline only with a scoped synchronization/presentation plan. Preserve feature scope and transparent fallback across backends.

Regular hosted CI checks code correctness on all three OSes. The supplied performance workflow is an executable baseline **foundation** on manually provisioned physical runners; it does not yet certify Foxide or enforce quantitative player regression gates. Benchmark completion adds Foxide adapters, full metric validation and same-machine historical comparisons before release publication is enabled. Keep baseline-version upgrades separate from Foxide regressions, and archive old suite/tool manifests for reproducibility.
