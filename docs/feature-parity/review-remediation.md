# Review remediation during feature parity

Work remains on PR #13. The video-site helper slice is preserved separately while these review findings are addressed.

- The identifying benchmark ZIP is removed from the PR tree. Publication tests scan remaining result text/archives and the local inventory. Raw comparison output is restricted to ignored results/local storage. The public default branch and historical commits retain the archive until separately merged/cleaned; no history rewrite or force push has been performed.
- Executable/DLL discovery stays beside the executable or inside its own tools subdirectory. Development dependencies require explicit overrides. Tests reproduce ancestor-directory planting; Unix PATH executability tests remain enabled on Unix runners.
- The fixed spawn-based startup window is replaced with a bounded internal-readiness wait and a full sampling window afterward. A deliberately delayed 4.5-second fixture, deadline expiry, invalid deadlines and warmup-origin handling are tested. Internal restart remains distinct from display TTFF, and the measurement profile changes to avoid mixing protocols.
- Standalone comparison-report imports are independent of test ordering. The offline installer test asserts installed bytes. Windows telemetry, manifest generation, source badges and the offline report checker now have behavior tests; CI invokes the report checker explicitly.
- Desktop callbacks can be accepted without a native engine: loading/filtering a queue, URL validation, transport/track controls, missing-engine errors and subtitle selection are exercised through the same UI callbacks.
- CI now executes real desktop startup/shutdown and normal/compact previews under coverage before imposing a 90% aggregate gate that includes desktop and GPU features. This is a required gate, not a claim that the new measurement has passed. No source files are excluded to raise coverage.
- Medal tests cover median selection, resolution ties, exact competition ranks through fourth place, all-source feature eligibility and rejection of a single invalid/mismatched trial. The reported set of 35 surviving mutants has not yet been independently reproduced; a full mutation score is not claimed.

The older 88ba499 formatting/missing-member finding is superseded by the later verified radio checkpoint. The radio checkpoint passed desktop/code checks on all platforms. Subsequent audit changes are being verified separately.
