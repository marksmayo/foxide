# Experimental Windows ANGLE rendering

Current status: visual playback works after explicitly requesting GLES 3.0. Earlier GLES 2.0 CPU figures below are INVALID as rendered-performance evidence: a pixel test observed no video. The fixed 4K run uses more CPU than the default path in initial repeats, while reporting fewer dropped frames. No CPU saving is currently established.

Set FOXIDE_ANGLE_RUNTIME to an absolute directory containing a compatible libEGL.dll and libGLESv2.dll pair, then launch foxide-desktop normally. DLL dependencies must reside in that directory or trusted Windows locations. Use only a trusted runtime: loading native libraries executes their code. No ANGLE binaries are redistributed by this repository.

This opt-in selects EGL before WGL and prefers dxva2 decoding with d3d11va-copy/software fallback. The default path retains auto-safe. Diagnostics expose the actual decoder; a fallback is not evidence of direct GPU decoding. Invalid explicit directories fail before window creation. The environment is read at startup and should remain fixed for the process lifetime.

Local HEVC experiment on merged PR6 baseline: original copy-back median 94.33% of one core; EGL plus explicit dxva2 trials 22.22%, 22.50%, 18.32% (median22.22%). Default window geometry was not normalized; these exploratory engine observations do not establish physical frame presentation or battery savings. ANGLE alone still selected d3d11va-copy and used121.31%; explicit decoder preference was necessary. Orientation, HDR, subtitle composition, other GPUs, AV1, and fallback behavior require broader acceptance before enabling by default.

The local prototype used an already-installed Plex ANGLE runtime solely for investigation. Distribution needs an independently pinned, licensed runtime and notices.

third_party/glutin-winit is upstream0.5.0 (MIT), with one opt-in preference adjustment in DisplayBuilder::with_preference. The small bootstrap dependency is vendored because Slint1.18.1 explicitly requests WGL-first and exposes no public EGL preference selector. Changes should be proposed upstream and the patch retired when Slint exposes this policy.

Integrated-build single-trial checks: HEVC21.30% CPU with dxva2; H.26411.38% with dxva2 and zero reported output drops; AV1116.56% with d3d11va-copy fallback. These confirm that AV1 remains a copy-back limitation. The normal non-opt-in release executable was restored after each experimental build.

Native acceptance on Intel Iris Xe / ANGLE D3D11: GLES 2.0 pixel probe failed (zero video pixels, GL errors); GLES 3.0 passed (249 upright, zero inverted, 153 distinct frames over five seconds, no GL errors). A separate window/engine transition probe passed pause/resume, exact seek, speed changes, resize, fullscreen enter/exit and subtitle selection; 219 upright samples, zero inverted, 152 distinct composited frames and no GL errors. These tests inspect final Slint pixels but do not certify physical scanout or battery efficiency. Mouse-driven menu acceptance remains pending.

Corrected GLES3 integrated-build HEVC comparison (three randomized repeats per mode, seed20261116, same executable and fixture, 8s sampling with2s warmup): default median CPU124.76% of one core and median final engine-reported output drops405; ANGLE median CPU146.15% and median final output drops104. These are cumulative engine counters over trials with different startup timing, not normalized displayed-frame rates. ANGLE increases CPU in this collection; no efficiency improvement is established. Raw trials are retained locally in results/local/windows-gpu-interop/verified-perf/.
