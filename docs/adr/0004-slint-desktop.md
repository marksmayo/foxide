# ADR 0004: Slint desktop UI and isolated rendering boundary

Status: accepted for the desktop preview; supersedes the egui recommendation in ADR 0002. Native GPU efficiency acceptance remains pending.

Use Slint 1.18.1 with its winit/femtovg OpenGL renderer. It supplies a declarative menu bar, keyboard navigation, accessibility, scaling and native file pickers without a WebView. The dark Foxide shell keeps VLC's discoverable Media/Playback/Audio/Video/Subtitles/View/Help organization with a quieter video stage, compact controls and an optional playlist.

`player-ui` owns declarative layout and a safe, testable `UiState` → `Effect` interface. Menus, keys and controls share dispatch. Playback controls wait for an engine snapshot before showing success. Slint-generated vtable code has a narrowly scoped unsafe-code allowance; hand-written UI code denies unsafe code. The normal policy library forbids it.

`player-render` is the private native boundary: dynamically loaded libmpv client ABI 2, a bounded command queue, worker-owned property/event access, and a Slint-thread render context. libmpv renders into a persistent OpenGL texture/FBO. Slint borrows that texture for compositing; the application does not download video pixels to CPU memory. Renderer teardown frees libmpv's context before the texture and library. Decoder and audio scheduling remain libmpv responsibilities.

This is an application-side GPU texture path, not proof of decoder-to-display zero-copy. `hwdec=auto-safe` is requested; diagnostics expose the actual decoder. Windows D3D11/ANGLE interop, macOS VideoToolbox interop, Linux VAAPI/NVDEC display integration, HDR, frame presentation and power must pass physical-machine tests. A copy-back decoder or software fallback cannot earn a zero-copy claim. Initial UI state polling is 200 ms and does not meet the <30 ms control acknowledgement aspiration.

Slint was chosen over egui for conventional menus/accessibility and a more maintainable designed shell; over iced for the available borrowed OpenGL texture integration; and over GPUI for the current cross-platform delivery scope. Revisit the renderer if physical-machine interop fails the acceptance gates. Keep policy independent of either toolkit.

Desktop remains an optional Cargo feature. Default workspace coverage covers safe policy, CLI and benchmark code, not the feature-gated native renderer or generated GUI. GUI build/preview CI and interactive validation are separate evidence, not substitutes for coverage. The 90% desktop coverage requirement is still open.

Distribution must audit the exact Slint license selected (GPL/commercial/Royalty-free terms), libmpv build license, bundled FFmpeg components, third-party notices and codec patents. Dynamic loading alone does not make a GPL-enabled mpv build LGPL. The locally installed development DLL is not a redistribution clearance.

References: [Slint rendering notifier](https://docs.slint.dev/latest/docs/rust/slint/struct.Window.html#method.set_rendering_notifier), [borrowed OpenGL texture](https://docs.slint.dev/latest/docs/rust/slint/struct.BorrowedOpenGLTextureBuilder.html), [mpv render API](https://github.com/mpv-player/mpv/blob/master/include/mpv/render_gl.h), [Slint licensing](https://slint.dev/license).
