# Feature medals and native UI measurements

The 2026-11-16 research expands the catalog from 30 broad features to 68 desktop feature events. Every documented supporting player receives gold; there is no silver or bronze for simply having a feature. Unknown claims remain unknown. Sources, platform restrictions, optional helper requirements and subscription conditions live beside each claim in [features.json](../benchmarks/features.json).

Chromecast output, AirPlay video output and AirPlay audio receiving are separate events. VideoLAN documents Chromecast in VLC 3.x; SMPlayer documents its browser-mediated Chromecast workflow. Apple's QuickTime guide documents AirPlay video output on macOS. Kodi's service documentation describes an AirPlay receiver with version restrictions. Generic mobile marketing does not establish desktop support. Foxide's new feature entries remain planned until accepted in the application.

Other events cover subtitle search, online video URLs, plugins, custom keys/layout, gestures, system media controls, mini players, media artwork, metadata libraries, SMB browsing, cloud storage, downloads, watch synchronization, live TV/PVR, web remotes, JSON IPC, HD audio passthrough, 360/3D video, capture, trimming, audio conversion, CD ripping/burning, tags, internet radio, rotation, ICC color management, shaders, debanding, frame stepping and playlist search.

## UI medal events

There are 46 action scenarios, each with six potential metric events (276 total): total action-to-state latency, native action dispatch latency, state observation after dispatch, process-tree CPU time, post-action RSS and nonnegative RSS increase. Dispatch and observation are complementary parts of total latency, not independent proof of display presentation. Observation includes accessibility API overhead and 10 ms polling. CPU and RSS are measured across this same action window; RSS is not a peak or leak measurement.

Beyond menus and file dialogs, scenarios include 1,000-item playlist search/sort/enqueue, focus traversal, minimize/restore, theme/DPI changes, rapid resizing, 100-position scrubbing, thumbnail hover, PiP, OSD fading, menus during 4K playback, and repeated menu/resize cycles. Playback, geometry and presented-frame scenarios without a verified adapter stay pending. Planned events do not earn medals.

The hosted Linux job creates an isolated X11/Openbox desktop at 1920×1080, DPI 96, English locale. It tries native AT-SPI actions for Foxide, VLC and SMPlayer, ten fresh-process repeats each. Original window bounds are retained; applications use default window sizes, so these are not equal-viewport results. A matching process identity, absent precondition, real native action, and unique visible postcondition are mandatory. All trials must verify before an entrant contributes measurements. A batch stops at its first verification failure, retains that failure, and emits no rows; repeating more failed launches cannot make the batch eligible. At least three repeats and two comparable entrants are required for performance medals. Failed or unavailable adapters remain visible and unranked.

Windows UIA and macOS AX adapters remain available through the dedicated interactive workflow; they require an appropriately configured isolated desktop. Hosted playback benchmarking runs on all three OSes, but virtual hardware does not certify physical display TTFF, watts, battery life or zero-copy hardware decoding. README tables distinguish feature medals, playback fields and native UI fields, with source evidence and raw medians.

## Primary research

- [VideoLAN VLC 3 release](https://images.videolan.org/vlc/releases/3.0.0.html)
- [SMPlayer first steps](https://www.smplayer.info/en/first-steps)
- [QuickTime playback guide](https://support.apple.com/en-qa/guide/quicktime-player/qtp6cee0761b/mac)
- [Kodi AirPlay services](https://kodi.wiki/view/Settings/Services/Airplay)
- [IINA features](https://www.iina.io/)
- [MPC-HC upstream](https://github.com/clsid2/mpc-hc)
- [KMPlayer PC](https://www.kmplayer.com/pc)
- [Infuse](https://firecore.com/infuse)
- [Plex desktop downloads](https://support.plex.tv/articles/downloads-on-desktop/)
- [foobar2000](https://www.foobar2000.org/)
- [mpv manual](https://mpv.io/manual/stable/)

The full per-claim source list is in the catalog. Support is a documentation finding, not a successful runtime acceptance test on every machine.
