# Desktop and report performance follow-up

Base: `013c95b393a02ea8db5fe47500c876978b9e08e3`. Changes are on `perf/reduce-desktop-and-report-overhead`, for review through a PR.

The review covered the Rust CLI, playback policy, folder ordering, desktop refresh/native renderer, benchmark sampling, accessibility adapters, evidence rules and report generation. Accepted changes reduce allocations or repeated work while preserving existing output. This is an additional review, separate from the earlier 200-candidate ledger.

| Area | Implemented reduction | Evidence / expected benefit |
|---|---|---|
| Comparison report | Index trials by fixture/player once instead of rescanning the complete trial list for every pair | Synthetic 4,800-trial report: 207.29 → 14.52 ms median; identical complete summaries |
| Folder ordering | Keep the owned natural-sort chunk buffer; trim ASCII leading zeros in place | Isolated key-generation test: 122.11 → 99.15 ms median; 2,480,000 → 1,700,100 allocation/reallocation calls (31.4% fewer) |
| Accessibility lookups | Normalize the requested label once per `find`/`observe` operation | 5,000-node lookup: 1.508 → 0.879 ms median; ownership, visibility, normalization and ambiguity tests pass |
| Desktop polling | Pass static `CStr` names directly to libmpv for fixed properties | Removes repeated property-name heap allocations at the existing 5 Hz snapshot cadence; wall-time savings unmeasured |
| Track discovery | Construct dynamic property names directly as owned C strings, without cloning a shared prefix and then copying it again | Fewer temporary allocations per track; same property names and finite ID parsing |
| Track labels | Query language only when the title is empty | Removes an unused native property query for titled tracks; same displayed fallback label |
| Engine startup | Use static C strings for the seven fixed option pairs | Removes 14 startup string allocations; option values unchanged |
| Engine commands | Reserve space for the trailing null pointer before building the command array | Avoids growing a full pointer vector when appending its terminator |
| Track-cache path | Reuse the existing string allocation with `clone_from` | Avoids repeated path allocations when refreshing an empty track list |
| Desktop title | Update only when the title changes or playback first loads | Avoids repeatedly copying the same title into Slint |
| Desktop clock | Format time labels only when their displayed whole second changes | Typically one formatted time label per second instead of five during playback |
| Desktop duration | Format the duration label only when its displayed value changes | Avoids repeating an unchanged label throughout a normal file |
| Desktop diagnostics | Rebuild text only at initial population or when decoder/error changes | Avoids formatting a long unchanged diagnostic string on every position update |
| Medal reporting | Remove an unused measurement index | Removes redundant list entries while preserving all grouping and evidence rules |

The timings are local microbenchmarks, not player startup, FPS, battery, or whole-application speedup measurements. Python timings are medians of seven repetitions on synthetic in-memory inputs; accessibility timings exclude native tree acquisition. Sort timing is the median of five runs, each computing keys for 1,000 Unicode episode names 100 times. The sort harness counted both allocation and reallocation calls with a forwarding system allocator. Before/after key outputs and complete report summaries were compared exactly. These workloads establish the local effect of each change, not general player superiority.

Validation includes default Rust regression tests, Clippy with the desktop feature, a real Slint state-refresh test using an offscreen window adapter, Python tests, and exact replay of 579 archived measurement rows across three platform sessions. The desktop workflow now runs the offscreen refresh regression test on all three platforms. Default-feature Rust coverage and policy coverage remain separately scoped from native GPU acceptance.

## Changes deliberately excluded

No changes to hardware-decoder choice, GPU texture ownership, synchronization, render-thread scheduling, GL state restoration, polling cadence, sampler identity checks, or medal eligibility. Those require stronger platform-specific evidence. Additional track caching was excluded because late track changes need reliable invalidation. Caching transient status messages was excluded because it could change how command errors and opening messages clear. Replacing min/max or median calculations was excluded to avoid changing exceptional numeric behavior.

## Reported slow desktop playback

The user reported slow playback during this review. The desktop-control inventory exposed no Foxide window; process metadata queries did not establish a running instance, so the symptom could not be reproduced here. Clarification is pending on whether audio/video run slowly, video alone stutters, or controls lag, and whether the debug or release binary is running. These refactors must not be described as a confirmed fix for that report. Native frame timing and decoder evidence are still required before changing the rendering pipeline.
