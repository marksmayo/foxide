# ADR 0003: Require GPU-resident decode-to-presentation and versioned interop

Status: proposed, subject to per-platform GPU/capture evidence.

## Definitions

For Foxide, a verified **GPU-resident path** means decoded surfaces remain on the GPU through color processing, subtitle/OSD composition and presentation, with no CPU pixel readback/reupload. Literal zero-copy scanout/overlay is a stricter separate result. GPU-to-GPU copies and color conversion passes still consume bandwidth and power; record their counts/sizes when observable. `hwdec=auto-safe` is a selection policy, not proof of this path or a power guarantee.

## Portable first spike

For a pinned OpenGL-capable libmpv release, create the window/context first, make it current on its designated render thread, create the libmpv render context with the GL procedure resolver, and only then load media. Supply the window/default framebuffer or an application-owned FBO with matching size/format. On render wakeup, update/render the pending video frame, reset state and draw the latest egui mesh; swap once and provide supported swap feedback. Render wakeups must only notify/signal, never render recursively. Do not invoke `glReadPixels`, `av_hwframe_transfer_data` into CPU memory, or a UI RGBA image-upload loop for the supported playback path.

The checked [mpv 0.41 render header](https://github.com/mpv-player/mpv/blob/v0.41.0/include/mpv/render.h) lists GL and software APIs. [Newer upstream render headers](https://github.com/mpv-player/mpv/blob/master/include/mpv/render.h) also expose D3D11. These are different capability sets; pin a commit/release and check exported symbols/client ABI plus render-context creation success. Do not infer public Vulkan/Metal render APIs from the CLI's `gpu-api` choices. The CLI's own renderer and libmpv embedding APIs have different integration contracts.

## Platform paths and gates

| Platform | Candidate path | Required evidence / fallback |
|---|---|---|
| Windows | Prefer application-owned D3D11 device, D3D11VA and libmpv D3D11 render API where available; shared swapchain target plus D3D11 OSD | Pinned headers/library support; multithread-protected device, serialized immediate context, same adapter; trace for staging/readback and cross-adapter copies. Portable GL/ANGLE path is a separate measured alternative, not assumed equivalent. |
| macOS | VideoToolbox decode with validated CVPixelBuffer/IOSurface-to-GL interop for initial spike | Verify actual import/copies on Apple Silicon and Intel separately. No assumed libmpv Metal backend. If it misses the power gate, investigate a native VideoToolbox/Metal or GStreamer/native presenter. |
| Linux | VAAPI dma-buf/EGL interop or NVDEC GPU sharing with the selected GL driver/context | Validate driver/GPU/Wayland/X11 combinations, sync fences and buffer formats. Treat software and `*-copy` decode modes as explicit fallbacks. |

The upstream [D3D11 render header](https://github.com/mpv-player/mpv/blob/master/include/mpv/render_d3d11.h) specifies an application-supplied device/target texture and immediate-context restrictions. Its device must permit multithreaded decode use; do not use the single-threaded creation flag. libmpv rendering changes GPU state, so the UI painter must restore its own state before drawing. Resource sharing across different devices needs explicit synchronization; prefer one device on the display-driving low-power GPU. This path is not implemented in the bootstrap.

GL paths retain context-current and surface-lifetime invariants across resize, teardown and device loss. Inspect each hwdec/interop combination instead of forcing a GL option on a device that cannot import its decoder surfaces. CPU fallback remains available for unsupported formats and is reported to diagnostics. Avoid continuously waking the discrete GPU on hybrid laptops.

## Validation and decision gate

Record active decoder, interop, adapter, target format/HDR mode and build provenance for every acceptance run. Use GPU traces to check CPU-visible staging/map/readback and device copies, then quantify matched ten-minute whole-system/package power against native players. Validate color, motion, subtitles and drops as part of the same gate. Hardware decode with no pixel readback can still lose on power due to shader complexity, compositing, polling or GPU choice. Revisit backend/presenter if the confidence interval fails the predeclared native-power tolerance; do not declare zero-copy from a configuration value alone.
