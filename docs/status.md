# Foxide engineering status

Foxide is a usable Rust launcher around external mpv, with validated playback options, local/network playlists, natural folder queueing, synchronization, picture controls, watch-later resume, and minimal-window options. See [capabilities](features.md). Embedded libmpv and the custom GPU window/OSD remain future work.

[Player Games](player-games.md) provides a sourced 30-event feature comparison, 45 performance protocols, three platform boards, HTML, SVG graphs, and generated README tables. Verified playback and startup-only diagnostics remain distinct. Physical collectors remain pending.

Local mpv came from the official pinned archive with verified SHA-256. Windows H.264 playback reported D3D11VA and GPU output. Eight additional competitor copies under `tools/` passed process startup checks. See [local inventory](local-players.md); startup does not certify playback.

The Rust workspace measures above 90% production-source line coverage with `cargo llvm-cov`, enforced by CI. Medal/report policy modules have a separate 90% statement/branch coverage gate. Stable Rust line coverage does not imply branch coverage. Native GUI, codec/rendering correctness, hardware/power acceptance, and host orchestration have separate runtime validation requirements.

Repository: [marksmayo/foxide](https://github.com/marksmayo/foxide). Consult live Actions outcomes/artifacts for exact status; workflow definitions alone are not evidence of a successful benchmark.

Remaining milestones: audited libmpv FFI, Rust/GPU surface sharing, OSD/accessibility, thumbnail workers, native PiP, library database, network resilience, complete frontend control adapters, and physical TTFF/seek/power collectors. See the [ADRs](adr/0001-media-backend.md), [design](architecture.md), and [roadmap](roadmap.md).
