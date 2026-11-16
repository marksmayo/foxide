# Six paired Windows renderer trials

All twelve trials verified engine playback. Each pair compares the same executable and HEVC fixture, with three pairs running ANGLE first and three running default first. Pair order was randomized (seed 20261116). No builds or tests ran alongside collection. Each trial collected 30 seconds after readiness, excluding a five-second warmup.

| Metric | Default | ANGLE |
|---|---:|---:|
| Mean CPU (% of one logical core) | 103.45 | 154.56 |
| Median CPU | 103.38 | 154.82 |
| CPU sample standard deviation (percentage points) | 7.77 | 1.78 |
| Mean steady RSS (MiB) | 1165.11 | 1194.59 |
| Mean output drops per engine second | 42.06 | 2.04 |
| Median output drops per engine second | 45.13 | 0.06 |

ANGLE used more CPU in all six pairs: mean paired increase 51.11 percentage points, approximately 49.41% above default. Mean RSS was 29.48 MiB higher.

The decoder paths were d3d11va-copy for default and direct dxva2 for ANGLE. ANGLE reported substantially fewer output drops. Drop rates use counter deltas over the steady observation window, rather than final lifetime totals. They are engine observations, not physical scanout/FPS measurements. Different delivered work prevents interpreting lower CPU alone as better efficiency. ANGLE's drop rates also varied: median 0.06/s versus mean 2.04/s.

These results reproduce the CPU increase on the quieter local machine; they do not establish a zero-copy battery benefit. Retain the opt-in/draft status while investigating CPU cost and frame delivery together. This collection is not added to the competitive medal scoreboard.

| Pair | Default CPU | ANGLE CPU | Difference (percentage points) |
|---|---:|---:|---:|
| 1 | 103.90 | 156.25 | +52.35 |
| 2 | 102.86 | 154.51 | +51.65 |
| 3 | 89.88 | 152.14 | +62.26 |
| 4 | 101.98 | 155.13 | +53.15 |
| 5 | 111.53 | 152.83 | +41.29 |
| 6 | 110.53 | 156.52 | +45.99 |

[Structured summary and hashes](2026-11-six-pair-renderers.json). The same local GLES3 build previously passed composited orientation/cadence and engine/window transition checks. Binary source description and exact executable, libmpv, ANGLE and fixture hashes are recorded in the JSON; no exact source commit is asserted for this local build. The collector and raw local trials are retained under results/local/windows-gpu-interop/. Default window geometry was used; this was not a controlled physical display or power test.
