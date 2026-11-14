
## Player medal scoreboards

Feature support earns **shared gold** for every documented supporter. Unknown is unverified, not unsupported. Counts reflect the documentation reviewed and do not establish an overall best player. Performance medals require at least three verified, comparable repeats on the same platform, host, fixture, scenario, and settings.

Results are readable directly on GitHub below. [Download optional HTML](./report.html) to open locally · [Raw scores](./medals.json) · [Feature sources](./features.json) · [Performance protocols](./performance-events.json)

UI events include menus, dialogs, playback controls, scrubbing, fullscreen and resizing. Native UI acknowledgement has a separate cohort from displayed-frame timing. Missing adapters or presentation evidence remain unranked. [UI benchmark protocol](./../ui-benchmarks.md).

### Windows

![Windows feature golds](./windows.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 3 | 9 | 2 | Verified playback 18/21 |
| VLC | 20 | 2 | 1 | 8 | Verified playback 15/21 |
| mpv | 26 | 10 | 3 | 1 | Verified playback 18/21 |
| Windows Media Player | 2 | 0 | 0 | 0 | installed-adapter-needed |
| Windows Media Player Legacy | 1 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| MPC-HC | 10 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| MPC-BE | 19 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| PotPlayer | 13 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| SMPlayer | 9 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| KMPlayer | 9 | 0 | 0 | 0 | not-detected |
| GOM Player | 10 | 0 | 0 | 0 | installed-adapter-needed |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Plex | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| foobar2000 | 6 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/e04828eb27f83752 | ms | 🥈 103 | — | 🥇 **88.2** |
| Paused video CPU / vm-software-null-output/paused/94639af1b8ea8a22 | % of one core | 🥇 **0** | — | 🥇 **0** |
| Paused video steady RAM / vm-software-null-output/paused/94639af1b8ea8a22 | MiB | 🥈 90.7 | — | 🥇 **85.7** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/e04828eb27f83752 | % of one core | 🥉 25.7 | 🥈 25.2 | 🥇 **24.1** |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/e04828eb27f83752 | MiB | 🥈 108 | 🥉 140 | 🥇 **104** |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/26dbe62c8f570fef | % of one core | 🥇 **158** | 🥉 184 | 🥈 175 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/26dbe62c8f570fef | MiB | 🥈 348 | 🥉 732 | 🥇 **344** |
| 4K AV1 CPU / vm-software-null-output/av14k/7ad23dc74fb1aa19 | % of one core | 🥈 107 | 🥇 **104** | 🥉 114 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/7ad23dc74fb1aa19 | MiB | 🥈 478 | 🥉 555 | 🥇 **473** |
| FLAC audio CPU / vm-software-null-output/flac/af777a91cf88831c | % of one core | 🥈 1.06 | 🥉 1.54 | 🥇 **0.794** |
| FLAC audio steady RAM / vm-software-null-output/flac/af777a91cf88831c | MiB | 🥉 55 | 🥇 **25.2** | 🥈 50.9 |
| 2× playback CPU / vm-software-null-output/speed2x/1b113898b09c7c40 | % of one core | 🥇 **45.7** | 🥉 54.1 | 🥈 47.8 |
| 2× playback steady RAM / vm-software-null-output/speed2x/1b113898b09c7c40 | MiB | 🥈 110 | 🥉 140 | 🥇 **105** |
| Peak playback RAM / vm-software-null-output/avc1080/e04828eb27f83752 | MiB | 🥈 109 | 🥉 140 | 🥇 **104** |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

| Feature | Foxide | VLC | mpv | Windows Media Player | Windows Media Player Legacy | MPC-HC | MPC-BE | PotPlayer | SMPlayer | KMPlayer | GOM Player | FFplay | Kodi | Plex | foobar2000 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? | N/A |
| Audio track selection | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 | 🥇 | N/A |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | N/A |
| External subtitle files | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A |
| Variable playback speed | 🥇 | 🥇 | 🥇 | ? | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? |
| Playlists | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | N/A |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | N/A |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | N/A |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | ? | ? | 🥇 | ? | N/A |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | 🥇 | ? | 🥇 | ? | 🥇 | ? | N/A |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | N/A |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | 🥇 | ? | ? |
| Native picture in picture | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | N/A |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | ? | ? | ? | 🥇 |
| Persistent playback resume | 🥇 | ? | 🥇 | ? | ? | 🥇 | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? | 🥇 | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | N/A |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | 🥇 | ? | ? | ? | N/A |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

### macOS

![macOS feature golds](./macos.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 3 | 8 | 3 | Verified playback 18/21 |
| VLC | 20 | 9 | 0 | 2 | Verified playback 15/21 |
| mpv | 26 | 2 | 6 | 6 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | not-detected |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Plex | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| foobar2000 | 6 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Infuse | 12 | 0 | 0 | 0 | not-detected |
| IINA | 27 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| QuickTime Player | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/783e332d3096ad53 | ms | 🥈 174 | — | 🥇 **159** |
| Paused video CPU / vm-software-null-output/paused/0bea2801485d4ab6 | % of one core | 🥇 **0.956** | — | 🥈 1.03 |
| Paused video steady RAM / vm-software-null-output/paused/0bea2801485d4ab6 | MiB | 🥈 172 | — | 🥇 **164** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/783e332d3096ad53 | % of one core | 🥈 28.6 | 🥇 **3.14** | 🥉 51.2 |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/783e332d3096ad53 | MiB | 🥈 217 | 🥇 **95.6** | 🥉 219 |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/7e2fcf4d2dbe83e0 | % of one core | 🥉 209 | 🥇 **6.04** | 🥈 191 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/7e2fcf4d2dbe83e0 | MiB | 🥈 520 | 🥇 **124** | 🥉 521 |
| 4K AV1 CPU / vm-software-null-output/av14k/cc549dfb583cc5cd | % of one core | 🥇 **109** | 🥉 118 | 🥈 117 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/cc549dfb583cc5cd | MiB | 🥇 **632** | 🥉 689 | 🥈 634 |
| FLAC audio CPU / vm-software-null-output/flac/7de95cd24bb6c63f | % of one core | 🥈 2.58 | 🥇 **2.44** | 🥉 6.55 |
| FLAC audio steady RAM / vm-software-null-output/flac/7de95cd24bb6c63f | MiB | 🥉 172 | 🥇 **67.6** | 🥈 170 |
| 2× playback CPU / vm-software-null-output/speed2x/c8eb09adbcf8aeaa | % of one core | 🥈 52 | 🥇 **17.8** | 🥉 78.1 |
| 2× playback steady RAM / vm-software-null-output/speed2x/c8eb09adbcf8aeaa | MiB | 🥉 218 | 🥇 **97.5** | 🥈 212 |
| Peak playback RAM / vm-software-null-output/avc1080/783e332d3096ad53 | MiB | 🥈 230 | 🥇 **96.8** | 🥉 233 |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

| Feature | Foxide | VLC | mpv | SMPlayer | FFplay | Kodi | Plex | foobar2000 | Infuse | IINA | QuickTime Player |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | N/A | 🥇 | 🥇 | ? |
| Audio track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | N/A | 🥇 | 🥇 | ? |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | N/A | 🥇 | 🥇 | 🥇 |
| External subtitle files | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | 🥇 | 🥇 | ? |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| Variable playback speed | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | 🥇 | 🥇 | 🥇 |
| Playlists | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | 🥇 | 🥇 | ? | 🥇 | ? |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | 🥇 | 🥇 | ? |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | ? | 🥇 | ? |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? | N/A | ? | 🥇 | ? |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? | N/A | ? | 🥇 | ? |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 | ? | ? | 🥇 | 🥇 | ? |
| Native picture in picture | ? | ? | ? | ? | ? | ? | ? | N/A | 🥇 | 🥇 | 🥇 |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Persistent playback resume | 🥇 | ? | 🥇 | 🥇 | ? | ? | ? | ? | ? | 🥇 | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | ? | ? | ? | 🥇 | 🥇 | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? | ? | ? | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | N/A | ? | 🥇 | ? |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? | ? | ? | ? | 🥇 | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? | ? | 🥇 | ? | 🥇 | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? | ? | N/A | ? | ? | ? |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? | ? | ? | 🥇 | ? | 🥇 |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

### Linux

![Linux feature golds](./linux.svg)

| Player | Feature 🥇 | Performance 🥇 | 🥈 | 🥉 | Runtime outcome |
|---|---:|---:|---:|---:|---|
| Foxide | 23 | 4 | 7 | 3 | Verified playback 18/21 |
| VLC | 20 | 4 | 0 | 7 | Verified playback 15/21 |
| mpv | 26 | 7 | 6 | 1 | Verified playback 18/21 |
| SMPlayer | 9 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| FFplay | 4 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |
| Kodi | 7 | 0 | 0 | 0 | Unverified / diagnostic (3 cases) |

Measured competitive event cohorts: **14**. Zero performance medals may mean measurements are pending. VM decode results never establish displayed TTFF, GPU efficiency, or battery performance.

<details><summary>Every measured metric — median values and podiums</summary>

| Metric / cohort | Unit | Foxide | VLC | mpv |
|---|---|---:|---:|---:|
| Engine playback restart latency / vm-software-null-output/avc1080/86b43f89289300e5 | ms | 🥇 **153** | — | 🥈 153 |
| Paused video CPU / vm-software-null-output/paused/4109bf58eb7c5e1f | % of one core | 🥇 **0** | — | 🥇 **0** |
| Paused video steady RAM / vm-software-null-output/paused/4109bf58eb7c5e1f | MiB | 🥈 103 | — | 🥇 **100** |
| 1080p H.264 CPU / vm-software-null-output/avc1080/86b43f89289300e5 | % of one core | 🥇 **33** | 🥉 34.1 | 🥈 33.3 |
| 1080p H.264 steady RAM / vm-software-null-output/avc1080/86b43f89289300e5 | MiB | 🥈 120 | 🥉 159 | 🥇 **117** |
| 4K60 HEVC 10-bit CPU / vm-software-null-output/hevc4k/30a9256e4b744531 | % of one core | 🥈 190 | 🥇 **184** | 🥉 192 |
| 4K60 HEVC 10-bit steady RAM / vm-software-null-output/hevc4k/30a9256e4b744531 | MiB | 🥈 388 | 🥉 452 | 🥇 **386** |
| 4K AV1 CPU / vm-software-null-output/av14k/26bb47ccf49f4e14 | % of one core | 🥉 173 | 🥇 **168** | 🥈 169 |
| 4K AV1 steady RAM / vm-software-null-output/av14k/26bb47ccf49f4e14 | MiB | 🥈 486 | 🥉 571 | 🥇 **483** |
| FLAC audio CPU / vm-software-null-output/flac/c7d7fc662ec00a63 | % of one core | 🥇 **0.504** | 🥉 0.813 | 🥈 0.673 |
| FLAC audio steady RAM / vm-software-null-output/flac/c7d7fc662ec00a63 | MiB | 🥉 67.8 | 🥇 **38.1** | 🥈 65.3 |
| 2× playback CPU / vm-software-null-output/speed2x/f02685815b83eb43 | % of one core | 🥉 69.3 | 🥇 **67** | 🥈 69.2 |
| 2× playback steady RAM / vm-software-null-output/speed2x/f02685815b83eb43 | MiB | 🥈 121 | 🥉 160 | 🥇 **119** |
| Peak playback RAM / vm-software-null-output/avc1080/86b43f89289300e5 | MiB | 🥈 120 | 🥉 159 | 🥇 **118** |

Values require ≥3 verified trials; — means no eligible comparable result. Each row is a separate cohort, never a cross-host ranking.

</details>

<details><summary>Feature events — shared gold and documentation status</summary>

| Feature | Foxide | VLC | mpv | SMPlayer | FFplay | Kodi |
|---|---:|---:|---:|---:|---:|---:|
| MP4 and MKV playback | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? |
| FLAC audio | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Hardware decoding | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 |
| Audio track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| Subtitle track selection | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 |
| External subtitle files | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| ASS/SSA subtitle rendering | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Variable playback speed | 🥇 | 🥇 | 🥇 | 🥇 | ? | ? |
| Playlists | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Keyboard controls | 🥇 | 🥇 | 🥇 | ? | 🥇 | ? |
| Frame-accurate seeking | 🥇 | ? | 🥇 | ? | ? | ? |
| Seekbar thumbnail previews | ? | ? | ? | ? | ? | ? |
| HDR video playback | 🥇 | ? | 🥇 | ? | ? | ? |
| HDR to SDR tone mapping | 🥇 | ? | 🥇 | ? | ? | ? |
| Audio synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| Subtitle synchronization offset | 🥇 | 🥇 | 🥇 | 🥇 | ? | 🥇 |
| Video filters and adjustments | 🥇 | 🥇 | 🥇 | 🥇 | 🥇 | ? |
| Chapter navigation | 🥇 | 🥇 | 🥇 | ? | ? | 🥇 |
| Native picture in picture | ? | ? | ? | ? | ? | ? |
| Network stream playback | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Persistent playback resume | 🥇 | ? | 🥇 | 🥇 | ? | ? |
| Automatic folder playback | 🥇 | ? | ? | ? | ? | ? |
| Borderless viewing mode | 🥇 | ? | 🥇 | ? | ? | ? |
| Video screenshots | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| A–B repeat | 🥇 | 🥇 | 🥇 | ? | ? | ? |
| Audio equalizer / DSP | ? | 🥇 | 🥇 | ? | ? | ? |
| ReplayGain | ? | 🥇 | 🥇 | ? | ? | ? |
| Gapless audio playback | ? | ? | 🥇 | ? | ? | ? |
| DVD / Blu-ray structure playback | ? | 🥇 | 🥇 | ? | ? | ? |
| AirPlay / Cast output | ? | ? | ? | ? | ? | ? |

? = undocumented/unverified, not a claim of absence. N/A = inapplicable media type. Source links and notes are in the feature catalog above.

</details>

