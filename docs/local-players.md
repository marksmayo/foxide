# Local Windows player inventory

Checked in this workspace. Eight new copies under `tools/` came from official release archives or verified official installer payloads. SHA-256 hashes matched upstream release digests or Microsoft WinGet manifests. Each new app passed a three-second process startup check; these checks do not certify playback or medal eligibility.

| Player | Local state | Executable |
|---|---|---|
| Foxide | Executable present | `target/release/foxide.exe` |
| VLC | Executable present | `C:/Program Files/VideoLAN/VLC/vlc.exe` |
| mpv | Executable present | `tools/mpv/mpv.exe` |
| Windows Media Player | Store installation blocked by WinGet `0x8A150001` | — |
| Windows Media Player Legacy | Executable present | `C:/Program Files/Windows Media Player/wmplayer.exe` |
| MPC-HC | Executable present | `tools/mpc-hc/mpc-hc64.exe` |
| MPC-BE | Executable present | `tools/mpc-be/mpc-be64.exe` |
| PotPlayer | Executable present | `tools/potplayer/PotPlayerMini64.exe` |
| SMPlayer | Executable present | `tools/smplayer/smplayer.exe` |
| KMPlayer 64X 2026.10.27.9 | Project-local signed payload; playback adapter needed | `tools/kmplayer/KMPlayer64.exe` |
| GOM Player | Executable present | `tools/gom/GOM.exe` |
| FFplay | Executable present | `%LOCALAPPDATA%/Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-9.0.2-full_build/bin/ffplay.exe` |
| Kodi | Executable present | `tools/kodi/kodi.exe` |
| Plex | Executable present | `tools/plex/Plex.exe` |
| foobar2000 | Executable present | `tools/foobar2000/foobar2000.exe` |
| Infuse | Other platform | — |
| IINA | Other platform | — |
| QuickTime Player | Other platform | — |

Launch Kodi with `-p` for its local portable data directory. Foobar2000 has its portable-mode marker enabled. PotPlayer has a local settings INI. Plex may need account/library setup. Modern Windows Media Player needs Store installation from an interactive Windows session; IINA, Infuse, and QuickTime are macOS entrants.

Local WinGet still returns `0x8A150001` at startup. Hashes, version/startup probes and manifests are retained in `results/local/` and `tools/*/foxide-installation.json` (machine-local, ignored by Git).

KMPlayer 64X was obtained from the [official publisher page](https://www.kmplayer.com/pc64x) and extracted under `tools/kmplayer` after Windows denied the installer launch. Both installer and player executable have valid PandoraTV Authenticode signatures. This is a project-local payload, not a system-wide registered installation. [Version, official URL, SHA-256 and signature evidence](../benchmarks/kmplayer-windows.json).

KMPlayer launch diagnostics: one process stayed active for the eight-second sample, and two subsequent launches exited normally. Instance handoff is possible but not verified. [Raw launch and installation evidence](../benchmarks/runtime/windows-local.json). These probes do not establish video playback and earn no performance medals.
