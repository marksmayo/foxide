# Windows player comparison

Open **[report.html](report.html)** in a browser. The document contains its charts, styles and data; it works offline. Choose a media profile and metric to see the gold/silver/bronze podium. Use Print / save PDF for the currently selected view. [comparison.json](comparison.json) contains full measurement data and [summary.json](summary.json) contains computed category summaries.

The field covers ten representative Windows players, plus Foxide and FFplay as an extra reference. It is not a verified popularity ranking. All entries appear in the report, including unavailable and unranked ones.

## What actually ran

- VLC 3.0.24 and FFplay 9.0.2: three 20-second trials each on 1080p AVC/AAC, 4K60 HEVC/PQ/AC3, 4K60 AV1/Opus and FLAC. Five seconds of warmup per trial, 250 ms process sampling, approximately one-second GPU counter collection. Eight unscored priming trials and 24 scored trials in total.
- Windows Media Player Legacy: an eight-second diagnostic launch. Process memory/CPU and GPU activity were observed, but playback was not independently verified. A subsequent read-only exact-file ownership query returned Windows error 29. Its diagnostic numbers are visible and receive no medals.
- Foxide 0.0.1: an actual launch attempt failed because its external mpv dependency is unavailable. It is a development launcher, not an independently implemented player.
- mpv and the other selected third-party players were not detected. Modern Windows Media Player was not detected in the checked app/executable inventory. Terminal downloads were blocked by the unavailable configured proxy; browser app access timed out.

All tested media are hashed 75-second repeated synthetic fixtures. The HEVC performance variant omits subtitle streams and the video cases request subtitles off. Audio stays enabled at each player's defaults. Video window sizes of 1280×720 were requested for VLC/FFplay; output geometry, HDR appearance, background activity, thermal state and audio volume were not independently normalized. These are exploratory resource-cost observations, not quality-equivalent native-power certification.

## Ranking rules

Require three completed trials with playback evidence, matching media profile and run/warmup durations. Use the median of per-trial summaries, with minimum/maximum trial ranges. Exclude failed/unverified runs and priming. Missing is never zero. At least two eligible players are required to award medals; a third-place slot stays vacant when there is no third eligible entrant. Ties at displayed precision share the corresponding placement.

RAM/CPU and Windows dedicated/shared GPU allocation are measured. The warm-window diagnostic measures a visible process-owned window, including collection/polling delay; it is **not displayed TTFF**. GPU allocation is not proof of resident VRAM and shared allocations can overlap CPU RSS. FFplay's own reported dropped-frame counts are shown as diagnostics; no common displayed-frame census was collected across players. There is no overall winner or combined score.

Cold/warm displayed-video TTFF, displayed-frame seek latency, package Watts/battery, display pacing and complete installed-dependency footprint remain unranked. No synthetic performance values were substituted for them.

## Reproduce

From the repository root, with the starter generated fixtures present:

```powershell
python scripts/compare_players.py --output results/local/new-comparison --duration 20 --warmup 5 --repeat 3
python scripts/build_comparison_report.py results/local/new-comparison/comparison.json
node scripts/check-report.mjs results/local/new-comparison/report.html
```

Update `benchmarks/windows-players.json` with official player executable locations as they become available. Native packaged apps and unvalidated frontends need an adapter and playback evidence before they can enter the podium. The suite refuses to overwrite an existing comparison, so use a fresh directory for each session.

The report’s JavaScript/data bindings, four profile selectors, every metric podium, native diagnostics, offline operation and print callback pass the dependency-free runtime checks. Ten Python tests cover ranking eligibility/ties/missing data plus telemetry. Browser visual QA could not run in this session: browser access approval timed out, and isolated headless Chromium terminated with Windows IPC access-denied errors. No visual screenshot review is claimed.
