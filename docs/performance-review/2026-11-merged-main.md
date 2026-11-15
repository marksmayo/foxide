# Merged-main performance review

Baseline: [Player Games run 43740394329](https://github.com/marksmayo/foxide/actions/runs/43740394329), source `8abf3ea`.

## Correct the software benchmark before optimizing

The macOS HEVC table reported Foxide at 253% of one core and VLC at 9.51%. The archived VLC HEVC stderr explicitly says `videotoolbox decoder: Using Video Toolbox`. The existing `--avcodec-hw=none` option controls avcodec, not VLC's separate VideoToolbox decoder. Those figures do not establish a software-decoding advantage.

Software trials now request `--codec=avcodec,none` plus `--avcodec-hw=none` and collect verbose decoder selection logs. The `none` terminator disables decoder fallback in [VLC's module loader](https://github.com/videolan/vlc/blob/3.0.x/src/modules/modules.c). Eligibility requires the selected audio/video module to be avcodec and rejects hardware-decoder evidence. Missing or unfamiliar logs fail closed. Native playback keeps automatic decoder selection.

Report refresh also retires older VLC software medals without explicit decoder evidence, preserving raw diagnostics. New collection replaces each field independently. Unchanged hosted hardware, seed hashes, trial counts and sampling settings are required for interpreting the rerun; its scores are a corrected baseline, not a Foxide speedup.

Validation: three new tests failed before their corresponding fixes and passed afterward; all 101 Python tests passed. Replaying the old macOS log rejects it. A live Windows VLC HEVC probe verified software selection and advancing playback. Native macOS validation is delegated to the new Actions run.

## Largest reproduced native gap

A new local release build from `8abf3ea` was tested against the existing hash-verified comparison corpus: three randomized 8-second HEVC trials per player, excluding 2 seconds of warmup.

| Observation | Foxide Desktop | mpv |
|---|---:|---:|
| Median CPU, percent of one core | 94.33 | 25.38 |
| Median steady process-tree RSS, MiB | 1174.95 | 1171.64 |
| Observed decoder | d3d11va-copy | d3d11va |
| Verified engine-playback trials | 3/3 | 3/3 |

This is exploratory native evidence at default window geometry, not a normalized competition or physical display/battery measurement. Raw results remain machine-local at `results/local/main-benchmark-review/native-hevc/`.

The copy-back path is the leading explanation, not a proven attribution of the entire CPU gap. Other candidates are compositor scaling and frame scheduling. The [libmpv OpenGL API](https://github.com/mpv-player/mpv/blob/master/include/mpv/render_gl.h) documents ANGLE requirements for direct Windows decoding. Foxide's current Slint renderer supplies a native OpenGL context. Changing only `hwdec` cannot create missing graphics interoperability.

Next acceptance experiment: an isolated ANGLE/shared-D3D rendering prototype, checked against this baseline with identical window geometry, decoder evidence, orientation/cadence assertions and repeated playback tests. No speculative renderer switch is included here, and no native performance improvement is claimed.
