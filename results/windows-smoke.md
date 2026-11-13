# Initial Windows diagnostic smoke

Three warm-primed VLC **3.0.24 Vetinari** runs completed on Windows 11 x86-64 using the generated 15-second 1080p AVC/AAC fixture. Each run lasted eight seconds with two seconds excluded as warmup; sampling interval was 250 ms. The extra `--verbose=2` option captured decode diagnostics. These measurements validate the harness, not competitive performance or battery claims.

| Metric | Observed result |
|---|---:|
| Completed measured runs | 3 |
| Mean steady resident set (average of run means) | 350.48 MiB |
| Maximum sampled resident set across runs | 382.33 MiB |
| Mean process-tree CPU (100% = one logical core) | 8.16% |
| Display TTFF / seek presentation / pacing | Unmeasured |
| Package Watts / GPU utilization / VRAM | Unmeasured |

VLC logs show the H.264 decoder starting, D3D11VA selection and D3D11 surface allocation. This is engine diagnostic evidence; there was no content-aware screen observation or GPU transfer trace. Some Windows taskbar-thumbnail calls failed in the logs. The runner intentionally retains `process-sampled-playback-unverified` status for VLC because it cannot independently verify presentation.

Raw samples, stderr logs, command lines and the environment manifest remain in `results/local/vlc-verified-path/`. A compact [evidence summary](evidence/windows-smoke.json) records per-run metrics, fixture/tool hashes and raw-directory references without committing the media or full local traces. The printed process report is at `results/local/vlc-verified-path/comparison.md`.

An earlier trial passed Windows verbatim canonical paths (`\\?\C:\...`) to VLC and exited before the measurement ended. The runner rejected those trials. The external-player path conversion was corrected and covered by a Windows regression test; those failed raw artifacts remain under `results/local/vlc-smoke/` and are not included here. An obsolete `--volume` option used in that earlier diagnostic trial was also removed.

No controlled thermal/display/power session was established. These short samples cannot validate the <80 MB target, a player's long-run memory behavior or any Foxide-versus-VLC advantage. CLI mpv and PresentMon were not installed/on PATH in this session, and terminal downloads were blocked by an unavailable configured proxy. macOS-native/Linux competitors require their respective physical systems. Use the full protocol before populating the competitive scoreboard.
