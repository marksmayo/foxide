# Foxide desktop preview

Build with `cargo build -p player-ui --features desktop --bin foxide-desktop --release --locked`. Run `target/release/foxide-desktop` (`.exe` on Windows), optionally passing one or more media files. This is separate from the original external-engine `foxide` CLI. There is no Electron/WebView runtime.

Windows first loads `libmpv-2.dll` beside the executable, then checks `tools/libmpv/libmpv-2.dll` below its first three executable ancestors. Automatic discovery never searches the working folder. DLL dependencies use restricted Windows search flags. Set `FOXIDE_LIBMPV` to an explicit ABI 2 library to override discovery. `python scripts/install-libmpv.py` installs the pinned Windows development DLL without changing PATH or file associations. The CLI-only installer `install-mpv.py` does not provide this DLL. macOS: install mpv with Homebrew. Linux: install your distribution's libmpv2 package. The renderer dynamically loads the library, so it is not needed for a layout-only preview.

Media/Playback/Audio/Video/Subtitles/View/Help menus expose file and URL opening, stop, pause, seeks, speed, tracks, subtitle loading, volume/mute, playlist, fullscreen and playback diagnostics. Space toggles playback, arrows seek five seconds, F/F11 toggles fullscreen, Escape leaves fullscreen, Ctrl+O opens files, Ctrl+L opens a URL and Ctrl+P opens the playlist. Native pickers run off the UI thread. The engine owns audio/video sync; bounded commands keep it separate from UI state.

The application renders libmpv into a shared OpenGL texture, without application-side pixel readback. `hwdec=auto-safe` is a request. View → Playback information shows the active decoder; platform interop, HDR and battery parity remain unverified. A software fallback is not zero-copy hardware decoding. The current Windows OpenGL path may lack D3D11/ANGLE interop. Do not infer power savings from the window opening.

Current preview limitations: 200 ms state refresh, no drag-and-drop, no thumbnail scrubber, no native PiP, no persistent GUI resume, no auto-advance after EOF, no complete desktop coverage, and no native macOS/Linux acceptance yet. Repeated file-picker requests can open more than one dialog. Audio-only cover presentation and fully accessible modal focus still need acceptance checks. Keep using the existing CLI for options not yet exposed in the GUI.

For a reproducible layout image without launching a desktop or loading libmpv:

```powershell
.\target\debug\foxide-desktop.exe --preview-png results/local/foxide-ui-preview.png
```

That image exercises the real Slint layout with the software renderer, not video playback. The desktop build workflow produces this artifact per platform. [ADR](adr/0004-slint-desktop.md) and [UI benchmark protocol](ui-benchmarks.md) describe the remaining acceptance gates.
