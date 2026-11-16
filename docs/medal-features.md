# Desktop feature controls

The desktop preview provides these user-facing controls through the existing bounded engine command queue. They do not change normal startup playback settings.

| Feature event | Desktop entry point | Behavior and limits |
|---|---|---|
| Single-frame advance | Playback → Next frame / Previous frame; . / , | Uses frame-step/back-step. Video only; backward accuracy depends on the media seek index and decoder. |
| Two subtitle tracks | Subtitles → Secondary subtitles | Select a second track independently, or turn it off. Secondary ASS styling restrictions apply. |
| Rotation and flipping | Video → Rotation / Flip vertically / Flip horizontally | Explicit 0/90/180/270° rotation; named flip filters toggle independently. Filters can require software processing. |
| ReplayGain | Audio → ReplayGain | Off, track gain, or album gain from source metadata. Off remains the default. |
| Debanding | Video → Debanding | Explicit on/off controls. Enabling it can increase GPU work. |
| Audio EQ / DSP | Audio → Equalizer | Bass boost and voice clarity presets, plus reset. The preview owns this audio-filter chain; reset clears these audio filters. |
| Playlist search | Playlist sidebar → Filter playlist | Case-insensitive filename filtering, preserving original queue order and selection identity. |
| Dark theme | Default appearance | The existing dark shell and native-widget palette remain in use. This was already implemented; its missing feature claim is corrected after preview verification. |

Video-only menu controls require a loaded video track. The UI waits for engine confirmation of loaded media. Command errors remain visible instead of being overwritten by an unchanged playback status on the next position tick.

## Acceptance evidence

Public UiState action tests exercise each control, invalid values and loaded-state gating. A headless Slint test verifies playlist models, stable selection identity, audio-only video-control availability and status changes. A separate native probe constructs commands through that same public action/dispatch interface, runs them on real mpv with audio/video and two external subtitles, and verifies command acceptance and resulting properties. Frame-step acknowledgement is asynchronous and is observed before continuing.

~~~powershell
$env:FOXIDE_MPV = "C:\path\to\mpv.exe"
$env:FOXIDE_CONTROL_FIXTURE = "C:\path\to\video-with-audio.mp4"
cargo test -p player-ui --features native-controls --lib desktop_menu_commands -- --ignored --nocapture
~~~

The dedicated GitHub workflow supplies a deterministic fixture. This is engine-control conformance, not displayed-frame, sound-quality or hardware-power benchmarking. The compositor regression remains separate.

Reference: [mpv control and option semantics](https://mpv.io/manual/stable/).
