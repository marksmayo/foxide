# CodSpeed CPU regression suite

Foxide registers **279 benchmark cases** across six targets. The existing 263 cases are retained and desktop customization adds 16 cases. The [argument-expanded inventory](../benchmarks/codspeed-inventory.json) records the exact names exercised by the hosted smoke run.

| Target | Cases | Workloads |
|---|---:|---|
| [player-core / core](../crates/player-core/benches/core.rs) | 6 | Default/full/rejected mpv arguments, decoder classification, folder queueing |
| [player-core / policy](../crates/player-core/benches/policy.rs) | 63 | Speed limits, 0–128 subtitles, controls, validation failures, decoder names and external-player paths |
| [player-ui / shortcuts](../crates/player-ui/benches/shortcuts.rs) | 2 | Aggregate and unmapped-key checks |
| [player-ui / actions](../crates/player-ui/benches/actions.rs) | 136 | Optional controls, filtering, thirty actions in loaded/unloaded states, playlists, navigation boundaries, scrubbing, duration validation and shortcuts |
| [player-bench / statistics](../crates/player-bench/benches/statistics.rs) | 56 | p50/p95/p99, distributions/non-finite values, missing CPU samples, warmup exclusion, summarization and JSON processing |
| [player-settings / preferences](../crates/player-settings/benches/preferences.rs) | 16 | Configured shortcut lookup, preference JSON decoding/encoding and validated edits |
| **Total** | **279** | |

Percentiles cover 0–65,536 values; summarization covers 0–40,000 samples; playlists cover 0–4,096 entries. The 2,400-sample case represents ten minutes at 250ms intervals. JSON cases use 0, 40 and 2,400 samples. Sample contents and operation sequences are deterministic.

## Measurement scope

These benchmarks measure Rust CPU work: argument allocation/formatting, state transitions/effect generation, telemetry reductions, selection and JSON processing. UI cases exercise the production reducer without starting Slint. They do not establish menu-to-pixel latency, video FPS, decoding throughput, startup TTFF, GPU efficiency or power use. Those remain separate [player](player-games.md) and [native UI](ui-benchmarks.md) protocols.

Existing folder-queue cases include filesystem enumeration. CPU simulation does not characterize disk latency or kernel I/O; interpret them as warm-folder policy/allocation regressions. Path normalization follows the host implementation; Linux scores do not measure Windows-only path conversion.

## Fixture discipline

- <code>with_inputs</code> constructs fresh mutable state and telemetry before each measured operation. Playlist/sample generation and input JSON encoding are outside the corresponding measured regions.
- Inputs and outputs cross <code>black_box</code> boundaries. Returned effects, JSON buffers and summary fields remain observable.
- Assertions outside timed sections validate fixture contracts. Smoke execution checks every parameter combination once without collecting timing scores.
- Playlist opening consumes prebuilt lists. Reducer state resets each iteration so selection/stop actions cannot drift into a different workload.
- Folder fixtures refuse to overwrite existing directories. Cleanup checks the canonical target remains a direct child of the intended temporary directory.
- The CLI and benchmarks share the existing telemetry model through a library. Model behavior and the on-disk schema are unchanged.

## Run locally

Run the full fixture smoke test:

    cargo bench --workspace --benches --locked -- --test

To list functions without exercising inputs, use <code>--list</code>. Divan omits value-argument expansion in list mode; the smoke output contains the complete inventory.

For CodSpeed collection, follow the [official Divan integration](https://codspeed.io/docs/benchmarks/rust/divan):

    cargo codspeed build -m simulation
    cargo codspeed run

Actual collection/upload requires the supported CodSpeed runner environment. Native smoke runs and a <code>--cfg codspeed</code> compilation check validate registration/code paths; they are not uploaded CodSpeed scores.

## Continuous reporting

The [workflow](../.github/workflows/codspeed.yml) runs on PRs and pushes to main. It pins cargo-codspeed 6.0.0 to match the current compatibility crates, smoke-runs all cases, saves the expanded inventory in the job summary/artifact, builds instrumented targets and uses CodSpeed's simulation action. OIDC remains enabled; no token is added.

If a job fails before any steps execute, no benchmark ran and no new score exists. Inspect its Annotations message to distinguish runner provisioning, account/repository restrictions and workflow startup errors. Empty logs alone do not identify the cause.

The harness also records a child that exits during final telemetry collection as incomplete, preserving its exit code. Regression tests cover already-exited children and harness-owned termination of a live child. Protocol fixtures allow three seconds for startup under instrumentation; this changes no performance target.

CodSpeed uses Ubuntu 24.04 and Rust 1.100.0, matching the recorded comparison environment. The workflow archives codspeed-environment.json with OS-image, architecture, libc and toolchain versions; it does not record a hostname or arbitrary environment variables. Upgrade the benchmark environment deliberately with a new comparable baseline, since changing libc or toolchains can change unchanged-code results.
