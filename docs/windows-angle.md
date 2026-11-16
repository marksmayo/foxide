# Experimental Windows ANGLE rendering

Set FOXIDE_ANGLE_RUNTIME to an absolute directory containing a compatible libEGL.dll and libGLESv2.dll pair, then launch foxide-desktop normally. DLL dependencies must reside in that directory or trusted Windows locations. Use only a trusted runtime: loading native libraries executes their code. No ANGLE binaries are redistributed by this repository.

This opt-in selects EGL before WGL and prefers dxva2 decoding with d3d11va-copy/software fallback. The default path retains auto-safe. Diagnostics expose the actual decoder; a fallback is not evidence of direct GPU decoding. Invalid explicit directories fail before window creation. The environment is read at startup and should remain fixed for the process lifetime.

Local HEVC experiment on merged PR6 baseline: original copy-back median 94.33% of one core; EGL plus explicit dxva2 trials 22.22%, 22.50%, 18.32% (median22.22%). Default window geometry was not normalized; these exploratory engine observations do not establish physical frame presentation or battery savings. ANGLE alone still selected d3d11va-copy and used121.31%; explicit decoder preference was necessary. Orientation, HDR, subtitle composition, other GPUs, AV1, and fallback behavior require broader acceptance before enabling by default.

The local prototype used an already-installed Plex ANGLE runtime solely for investigation. Distribution needs an independently pinned, licensed runtime and notices.

third_party/glutin-winit is upstream0.5.0 (MIT), with one opt-in preference adjustment in DisplayBuilder::with_preference. The small bootstrap dependency is vendored because Slint1.18.1 explicitly requests WGL-first and exposes no public EGL preference selector. Changes should be proposed upstream and the patch retired when Slint exposes this policy.

Integrated-build single-trial checks: HEVC21.30% CPU with dxva2; H.26411.38% with dxva2 and zero reported output drops; AV1116.56% with d3d11va-copy fallback. These confirm that AV1 remains a copy-back limitation. The normal non-opt-in release executable was restored after each experimental build.
