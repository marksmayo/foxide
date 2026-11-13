# ADR 0001: Use libmpv behind a narrow Rust engine facade

Status: proposed, to be confirmed by v0.1 platform spikes and measured baselines.

## Context and alternatives

| Backend | Advantages for Foxide | Work and risks |
|---|---|---|
| libmpv via audited Rust FFI | Mature playback, sync, seeking, subtitles, filters, chapters and streaming; fastest path to broad compatibility | Native dependency/package size; API/version-specific render integration; hardware activation alone does not establish battery efficiency |
| GStreamer / gstreamer-rs | Pipeline model; hardware memory types and native platform elements; mature Rust bindings | Plugin discovery/distribution footprint, autoplug negotiation and subtitle/seek behavior to validate per platform |
| FFmpeg / ffmpeg-next plus own presentation | Full control over demux/decode, surface lifetime and scheduling | Must implement clocking, resampling, subtitle composition, seamless seeks, HDR/color, device loss, streams and security maintenance |

The upstream [libmpv examples](https://github.com/mpv-player/mpv-examples/tree/master/libmpv) distinguish window embedding from application-managed render APIs. GStreamer's [D3D11 sink](https://gstreamer.freedesktop.org/documentation/d3d11/d3d11videosink.html) documents D3D11-backed memory/presentation; its Rust bindings do not automatically guarantee a negotiated GPU-only pipeline. FFmpeg's [hardware decode example](https://github.com/FFmpeg/FFmpeg/blob/master/doc/examples/hw_decode.c) illustrates device-context handling and an explicit download path; Foxide would need to supply its own presentation path without that transfer.

## Decision

Prototype libmpv first. The facade exposes commands, immutable playback state, capabilities and presenter handles with explicit lifetime ownership. FFI error checking covers null handles, option failures, client ABI compatibility, CString lifetime, event/node cleanup, callback userdata lifetime and ordered teardown. Keep unsafe calls in a private audited module; the current workspace foundation contains no media FFI. The renderer must not call synchronous client APIs across the engine/render thread boundary.

Default requested decode mode is `hwdec=auto-safe`; log actual decoder and interop. Preserve software fallback and label it. Do not insert `cpal` or a second A/V master clock above mpv. Keep audio/video decode scheduling engine-owned. Do not expose decoder-internal queues as Rust extension points.

## Consequences and validation

This reduces initial playback work but makes renderer interop, native build provenance, licensing and footprint explicit release gates. Complete A/B/C/D compatibility; exact/keyframe seek and subtitle correctness; audio continuity; memory accounting; ten-minute matched native-player power tests. If a supported GPU cannot meet the no-readback or native-power tolerance with correct quality, prototype GStreamer/native presentation behind the facade and compare under the same harness before committing to a custom FFmpeg engine. No backend is asserted to have won yet.
