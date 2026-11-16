# Verification and fresh benchmark results

Measured source: 268ff8a0e8b86e9af7dcdef8ef0dc2421c076f23. [Full hosted run](https://github.com/marksmayo/foxide/actions/runs/43864616988) passed on Windows 2025, macOS 15 and Ubuntu 24.04; Linux native UI and report publication also passed. The generated README retains older, separately identified local Windows fields. The table below includes only this hosted run.

| Fresh field | Foxide performance gold | Silver | Bronze | Feature gold |
|---|---:|---:|---:|---:|
| Windows software/null | 2 | 10 | 2 | 31 |
| macOS software/null | 2 | 8 | 4 | 30 |
| Linux software/null | 5 | 7 | 2 | 29 |
| Linux native UI | 12 | 8 | 4 | 29 |

Feature counts are platform totals and must not be added across fields. Software/null playback is not a hardware-decoding, display, battery or HDR certification. Scores are ranks within comparable fields, not proof that a particular source change caused a gain.

## UI observations

The Linux native UI suite verified 100 trials, produced 600 metric rows and recorded five adapter-required cases. Foxide menu-action medians were about 89–108 ms; VLC was about 145–165 ms. Accessibility polling and action acknowledgement are included; these are not physical display latencies. Default window geometry was not identical.

## Regression checks and coverage

All implementation-source checks passed, including platform tests, desktop builds, compositor checks, native command acceptance and CodSpeed. The existing 233 CodSpeed cases are retained and 30 control/filter cases added. Optional command generation is separated from the existing action path; inline annotations preserve hot dispatch and shortcut performance.

CodSpeed reported 33 improved, 200 untouched and 30 new cases, with zero regressions and an aggregate +30.22% result. Its report warns that 33 significant changes used different hardware; five existing cases may be optimized out and two are syscall-dominated. This result is not a causal or player-wide speed claim.

Measured coverage from the implementation checks before the final inline-only shortcut adjustment: default Rust lines 95.51%; selected scoring/report Python policy 97.29%; full feature-gated desktop/GPU Rust lines 81.77%. All Python scripts combined were 51.13%. The default/policy gates pass; full application coverage remains below the requested 90% target. Coverage scopes are intentionally explicit.

Native playback acceptance sends 18 real mpv commands through the public UI dispatch path, including preservation of the primary subtitle track. Both Windows and Linux passed. This validates engine state, not physical audio/display fidelity. The Linux probe's JSON-empty-array serialization defect was fixed rather than bypassed.

## README layout and scoring

All podium tables and feature matrices precede every chart. Universal contests omitted by platform: Linux audio-track and subtitle-track switching; macOS subtitle-track switching; none on Windows. Eligibility and sourced support determine universality. Evidence remains in the catalog and omitted-event records, and omitted contests contribute no medals.

Seven new desktop features and the already-present dark theme are credited. The audit's other 49 feature integrations remain open; they require larger implementation and acceptance work. Two renderer lifecycle optimizations remain deferred pending GPU lifecycle evidence. No source media, rendering quality or eligibility rules were weakened to improve rank.
