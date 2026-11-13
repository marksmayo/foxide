# Foxide foundation status

## Windows comparison update

An [offline interactive HTML comparison](../results/windows-comparison/report.html) now contains 24 measured VLC/FFplay trials across four media profiles, category medals, RAM/CPU graphs, time series, dedicated/shared GPU allocation, a Windows Media Player Legacy diagnostic and a twelve-entry player availability table. Foxide's actual launch attempt is recorded as blocked by its missing mpv dependency. See [comparison notes](../results/windows-comparison/README.md) for eligibility rules, missing competitors, unmeasured display/power categories and verification limits. Windows GPU allocation and engine counters are now collected in the comparison script; actual VRAM residency, display timing and power remain outstanding.

The initial benchmark and architecture deliverable is implemented in this folder. The folder was empty and had no Git repository. No remote was configured, no commits were created, no app was published and no CI execution is claimed.

## Completed and verified locally

- Four-crate Cargo workspace: `player-core`, `player-ui`, `player-cli` (the `foxide` launcher), `player-bench`.
- Bench CLI for repeated mpv/VLC/direct-executable launches, user-attested cold cache, warm priming, existing native-process observations and Markdown reports.
- Embedded Python process-tree sampling with Windows native process/working-set/CPU APIs; mpv Lua internal startup/seek/drop probes included.
- Synthetic AVC, HEVC 10-bit/PQ with ASS+SRT, AV1 and three FLAC/cover fixtures generated; all six hashes/stream contracts verified. Also generated a 15-second AVC/AAC fixture for process runs.
- Three completed VLC diagnostic process trials, with raw logs and [recorded evidence](../results/windows-smoke.md). Early-exit trials were excluded and led to the Windows path compatibility fix.
- Rust release build; formatting; Clippy with `-D warnings`; six Rust tests (including three subprocess integration tests); five Python tests, including a real parent/child process-tree sampling test.
- Benchmark protocol/scripts and comparison template; three proposed ADRs; feature matrix, engineering design and v0.1–v1.0 roadmap; hosted CI and dedicated physical-baseline workflow definitions.

## Explicitly outstanding

The application is not yet an embedded Rust player. `foxide <file>` launches an external mpv executable. `player-ui` currently supplies only keyboard mapping and `player-core` supplies policy/command types. No custom GPU window, egui OSD, libmpv FFI, persistence or network UI exists.

The bench runner currently supplies process telemetry and mpv internal proxies. Content-aware displayed TTFF, first-correct-frame seek latency, GPU/VRAM, package Watts, display frame pacing and native-app launch/seek adapters are future work. Native apps can be sampled with `observe --pid`; that does not measure their startup. The external OS commands in the protocol capture raw diagnostics, without automated trace joins/imports. Foxide's numerical targets and native power parity have not been tested.

CLI mpv was absent and a configured terminal proxy was unavailable, so the mpv adapter/Lua probes could not be exercised against a real mpv build here. PresentMon was absent. WPR/typeperf were detected but no privileged/extended tracing session was started. The full 660-second corpus and ten-minute power trials were not run. Linux/macOS sampling implementations and workflow definitions require validation on those platforms. Self-hosted runner setup, native player packaging and full regression certification remain unconfigured.

## Next concrete milestone

Install a pinned CLI mpv build with recorded source/hash, exercise the internal startup/seek probes, then implement the missing content-aware presentation/power collectors and native adapters. Collect controlled same-machine competitor baselines before confirming the proposed ADRs. Begin v0.1 with an audited libmpv client wrapper and GPU render spike once those measurements can judge it. The detailed [roadmap](roadmap.md) defines each subsequent gate.
