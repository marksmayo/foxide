# Versioned benchmark evidence

These bundles preserve the inputs behind the README graphs and tables. Each JSON file contains the session, executable inventory, raw per-trial cases and measurement rows. Row source references point to a case in the same file (`#/cases/N`); `original_source` retains the collector's original case path.

| Bundle | Field |
|---|---|
| [windows-local-native-window.json](windows-local-native-window.json) | Foxide Desktop, mpv and VLC with native audio/video output |
| [windows-local-software-null.json](windows-local-software-null.json) | Foxide CLI, mpv and VLC with software decoding/null output; other Windows frontends as unranked diagnostics |
| [windows-hosted-software-null.json](windows-hosted-software-null.json) | Hosted Windows software/null output |
| [macos-hosted-software-null.json](macos-hosted-software-null.json) | Hosted macOS software/null output |
| [linux-hosted-software-null.json](linux-hosted-software-null.json) | Hosted Linux software/null output |

The November 15 local snapshot uses three repeats, eight seconds after engine readiness and two seconds of excluded warmup. Per-trial steady CPU/RSS means are reduced to medians across repeats. Peak RSS is a sampled peak including startup. CPU is percent of one logical core. Native window sizes/audio routes are player defaults, and background activity and cache state are uncontrolled. These are short resource comparisons, not displayed-frame timing or ten-minute battery tests.

Hosted and local fields remain separate. The hosted bundles retain their original source commits, artifact hashes, Actions run IDs and collection dates; they exercise the Foxide CLI software path rather than native desktop presentation. Feature scores are sourced documentation claims, separate from performance measurements.

Failed/incomplete attempts remain available outside the scored-bundle directory:

- [Incomplete software collection](../attempts/windows-software-incomplete-2026-11-15.json): missing ASS fixture stopped the first attempt; no partial trial was reused in the complete rerun.
- [Hosted attempt](../attempts/hosted-2026-11-15.json): all jobs failed before steps ran, producing no new hosted measurements. The startup failure's cause is undetermined.

To regenerate the graphs/tables from these bundles:

```powershell
python scripts/refresh_scoreboards.py --readme README.md
```

[Full protocol and collection commands](../../docs/player-games.md)
