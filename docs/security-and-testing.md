# Security and test follow-up

Verified locally on 2026-11-16; Windows, offline dependency cache. This is a targeted follow-up to the supplied reviews, not a new whole-repository security certification or test-suite score.

| Evidence | Result | Scope |
|---|---:|---|
| Rust tests | 29 passed | Default workspace features |
| Rust instrumented line coverage | 95.95% (1,065 / 1,110 lines) | Default features; not desktop/GPU |
| Python tests | 98 passed | unittest discovery, offline |
| Policy/report statement + branch coverage | 97% | Seven policy/report/publication modules; CI minimum 90% |
| Installer statement + branch coverage | 95% | mpv + libmpv installers; separate CI minimum 90% |
| All Python scripts, statement + branch coverage | 49.87% | Full scripts directory, including orchestration; no aggregate 90% claim |
| Windows native rendering | Compile + Clippy passed | desktop feature |
| Numeric evidence preservation | 43 JSON snapshots unchanged | Every numeric field compared before/after privacy filtering |

The expanded whole-script inventory deliberately exposes the orchestration and platform collector gaps that the former six-module report omitted. The desktop/GPU coverage artifact instruments the workspace with desktop and GPU integration features, including the ignored software-OpenGL compositor test under Xvfb. No desktop coverage percentage is claimed here. A successful software-OpenGL test does not certify physical GPU decoding or battery efficiency. The cadence probe starts its fixed five-second sample window at the first observed colored video frame, with a separate 15-second startup deadline. The threshold remains 75 distinct frames and zero inverted observations; black or frozen video still fails.

An independent SDL/libmpv control isolated an upstream scaler-table padding problem from Foxide and Slint. Failing runs contained finite computed filter coefficients and NaNs in unused LUT channels. CI builds a pinned media engine with a one-line zero-initialization patch and retains a separate verified NaN-padding negative control. Twelve normal compositor trials and the instrumented pass use this patched test dependency. Downloaded/system runtime binaries are not modified by this CI setup; their platform acceptance remains separate. [Evidence, patch and scope](../third_party/mpv/README.md).

## Security changes

- Automatic CLI discovery ignores the media working directory and explicitly resolves absolute PATH entries. Missing engines fail at an executable-relative path, avoiding Windows' implicit bare-command search. Explicit CLI/environment overrides remain supported.
- Windows desktop discovery prefers the DLL beside Foxide, then development paths anchored to executable ancestors. Restricted DLL search flags exclude the working directory for transitive dependencies too.
- All workflow action references use immutable commit SHAs. The coverage tool version is pinned, CodSpeed uses its public-repository tokenless upload mode, and no workflow requests OIDC token write permission. Dependabot proposes action updates through PRs.
- Published evidence and reports hide OS account paths and replace machine names with platform/origin labels. Numeric measurements, dates, hashes and existing profile IDs remain intact. Private originals are retained only in ignored local storage. Earlier Git revisions retain their original content; this PR does not rewrite history.

## Regression evidence

The planted CLI executable test failed before the discovery fix and passed afterward. Privacy tests also failed before publication filtering and before the fix for account names containing spaces. Offline installer tests verify changed hashes are rejected before extraction or execution, malicious ZIP paths/symlinks are refused, interrupted downloads and failed probes clean staging, and existing installs survive. Comparison tests reject absent playback evidence and corrupted fixtures.

Launcher fixtures now clean themselves up after each test, including the compiled fake engine. The telemetry scratch-directory helper no longer has a test-discovery name.

## Remaining work

Broaden orchestration, native collector, desktop lifecycle/error-path, accessibility and physical GPU coverage. Dependency advisories from the supplied review were not re-audited against a fresh advisory database in this follow-up; yanked/unmaintained dependency warnings remain to be reviewed.

Run the same default coverage gates with the commands in [.github/workflows/ci.yml](../.github/workflows/ci.yml). Desktop/GPU instrumentation is in [.github/workflows/video-render.yml](../.github/workflows/video-render.yml). Coverage JSON artifacts are uploaded per platform.
