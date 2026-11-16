# Feature parity implementation

Foxide starts this work at 49 feature events without gold. The complete [acceptance checklist](remaining.json) preserves every event. This first batch implements three desktop events; the other 46 remain queued. Full parity is not claimed.

[Measured verification and limitations](verification.md).

## Desktop customization

Open **View → Preferences** to switch dark/light skins, choose a six-digit RGB accent color, or add/remove keyboard bindings. Changes apply immediately after a successful save. Existing key assignments must be removed before reuse; Escape remains available for closing dialogs and leaving fullscreen. Open **Help → Keyboard shortcuts** for the current bindings. Text editing and open dialogs do not dispatch playback shortcuts.

Open **View → Compact music view** for a dedicated 560×280 logical-pixel music layout with transport, seek and volume controls. The playlist and playback state remain available; leaving the mode restores the previous window size. The mode persists across restarts. It does not claim album-art display, gapless playback or native picture-in-picture.

Preferences are saved to:

| Platform | Default file |
|---|---|
| Windows | %APPDATA%/Foxide/preferences.json |
| macOS | ~/Library/Application Support/Foxide/preferences.json |
| Linux | $XDG_CONFIG_HOME/Foxide/preferences.json, falling back to ~/.config/Foxide/preferences.json |

Use foxide-desktop --config PATH to choose a different file. Missing files use defaults. Invalid files are left untouched on startup and the preferences panel reports the error; an explicit save/reset can replace them. Writes use a synced sibling temporary file followed by replacement; a failed write leaves the active settings unchanged. There is no script or shell-command setting. Reset preferences is an explicit UI action.

The three event claims require the public preferences tests plus the headless desktop acceptance test on Windows, macOS and Linux. CI archives dark, light, preferences and compact previews. Native hardware playback measurements remain separate.

## Rendered previews

![Preferences editor with a custom light skin](images/preferences.png)

![Compact music view](images/compact.png)

Settings storage/validation lives in the independent player-settings crate. The existing player-ui playback-policy library and action benchmarks remain unchanged, so adding configuration code does not enlarge their compilation unit. Customization benchmarks run as a sixth target; all existing cases and thresholds remain intact.

## Implementation order

1. Desktop customization: editable keyboard bindings, light/dark skins with custom accents, compact music view.
2. Network playback tools: video-site URL integration and radio-directory browsing, with bounded network requests and dependency/provider acceptance fixtures.
3. Local playback experience: artwork, thumbnails, gapless transitions, system media controls, gestures/controller navigation and native PiP.
4. Shared media and server clients: WebDAV/SMB/NFS/UPnP browsing, authenticated Jellyfin/Emby/Plex libraries, offline downloads, metadata and watched-state integrations.
5. Advanced engine controls and authoring: color profiles/shaders, projection/stereo, passthrough, extensions, capture, trimming, conversion and tag editing.
6. Receiver and device integrations: Chromecast, AirPlay, live TV/PVR and optical media. Emulator/protocol tests accompany implementation, and physical-device acceptance is recorded separately.

Every item remains in the checklist until its frontend interaction, error/cancellation behavior and acceptance evidence are implemented. Credentials, receiver hardware, optical drives and display-specific acceptance are explicit dependencies, not automatic gold medals.

## Red/green evidence

- Editable shortcut JSON round-trip, persistence, custom appearance and compact persistence each failed before their implementation and now pass.
- The desktop acceptance test failed before the callbacks and properties existed, then passed through the same public callbacks the UI uses.
- A real F2 window-event test caught the missing native key encoding and passed after the mapping fix.
- A Unicode key test caught case expansion corrupting a saved single-character binding and passed after preserving Unicode characters while normalizing ASCII case.
- Tests cover malformed/oversized files, rejected edits, conflicting keys, repeat saves, save failure, defaults and restoration of the normal window size.

Sixteen customization CPU benchmark cases extend the existing suite without changing its existing cases or regression thresholds. They measure configured key lookup, preference decoding/encoding and edits; they do not claim menu-to-pixel latency or battery efficiency.
