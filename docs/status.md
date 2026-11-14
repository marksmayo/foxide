# Foxide engineering status

Foxide now has an optional Slint desktop preview embedding libmpv in its own OpenGL window, alongside the existing external-mpv CLI. Menus, playback controls, native file pickers and playlists are implemented. The local Windows build and layout render pass; a visible window is user-confirmed. Moving-video playback, native accessibility and platform GPU efficiency still need acceptance. See [desktop setup](desktop.md) and [capabilities](features.md).

[Player Games](player-games.md) provides a sourced 30-event feature comparison, 117 performance protocols (including 72 UI events), three platform boards, HTML, SVG graphs, and generated README tables. The completed performance run supplies 579 measurements. UI events and physical collectors remain pending; definitions do not count as results. [UI protocol](ui-benchmarks.md).

Local mpv came from the official pinned archive with verified SHA-256. Windows H.264 playback reported D3D11VA and GPU output. Eight additional competitor copies under `tools/` passed process startup checks. See [local inventory](local-players.md); startup does not certify playback.

The Rust workspace measures above 90% production-source line coverage with `cargo llvm-cov`, enforced by CI. Medal/report policy modules have a separate 90% statement/branch coverage gate. Stable Rust line coverage does not imply branch coverage. Native GUI, codec/rendering correctness, hardware/power acceptance, and host orchestration have separate runtime validation requirements.

Repository: [marksmayo/foxide](https://github.com/marksmayo/foxide). Consult live Actions outcomes/artifacts for exact status; workflow definitions alone are not evidence of a successful benchmark.

Remaining milestones: audited libmpv FFI, Rust/GPU surface sharing, OSD/accessibility, thumbnail workers, native PiP, library database, network resilience, complete frontend control adapters, and physical TTFF/seek/power collectors. See the [ADRs](adr/0001-media-backend.md), [design](architecture.md), and [roadmap](roadmap.md).
