# Review remediation during feature parity

Work remains on PR #13. The video-site helper slice is preserved separately while these review findings are addressed.

- The identifying benchmark ZIP is removed from the PR tree. Publication tests scan remaining result text/archives and the local inventory. Raw comparison output is restricted to ignored results/local storage. The public default branch and historical commits retain the archive until separately merged/cleaned; no history rewrite or force push has been performed.
- Executable/DLL discovery stays beside the executable or inside its own tools subdirectory. Development dependencies require explicit overrides. Tests reproduce ancestor-directory planting; Unix PATH executability tests remain enabled on Unix runners.
- The fixed spawn-based startup window is replaced with a bounded internal-readiness wait and a full sampling window afterward. A deliberately delayed 4.5-second fixture, deadline expiry, invalid deadlines and warmup-origin handling are tested. Internal restart remains distinct from display TTFF, and the measurement profile changes to avoid mixing protocols.
- Standalone comparison-report imports are independent of test ordering. The offline installer test asserts installed bytes. Windows telemetry, manifest generation, source badges and the offline report checker now have behavior tests; CI invokes the report checker explicitly.
- Desktop callbacks can be accepted without a native engine: loading/filtering a queue, URL validation, transport/track controls, missing-engine errors and subtitle selection are exercised through the same UI callbacks.
- CI now executes real desktop startup/shutdown and normal/compact previews under coverage before imposing a 90% aggregate gate that includes desktop and GPU features. This is a required gate, not a claim that the new measurement has passed. No source files are excluded to raise coverage.
- Medal tests cover median selection, resolution ties, exact competition ranks through fourth place, all-source feature eligibility and rejection of a single invalid/mismatched trial. A reproducible comparison/boolean-operator check now kills 48/48 targeted mutants across medal_events.py and publication.py; CI runs tests/mutation_check.py and archives its results. This is a scoped mutation check, not a full mutation score for every possible mutation.

The older 88ba499 formatting/missing-member finding is superseded by the later verified radio checkpoint. The radio checkpoint passed desktop/code checks on all platforms. Subsequent audit changes are being verified separately.

Local Python branch-inclusive coverage after these additions: publication 100%, medal_events 98%, session-manifest 91%, source_lines 96%, windows_metrics 94%. CI enforces at least 90% for each of the three newly covered maintenance modules. Hosted desktop/GPU coverage is still being measured.

The first hosted coverage entry-point run rejected a redundant --no-clean/--no-report combination; the workflow now uses --no-report alone. CodSpeed exposed additional tiny-input summary overhead; the implementation now computes peak and steady statistics in one pass and uses an inline empty-percentile wrapper. Regression thresholds and benchmark cases remain unchanged; new hosted results are required before declaring the regressions resolved.
