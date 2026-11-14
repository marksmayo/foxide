# ADR 0002: Choose egui, initially with a shared OpenGL presenter

Status: superseded by [ADR 0004: Slint desktop](0004-slint-desktop.md). This document preserves the original proposal.

## Evaluation

| Candidate | Fit | Decision |
|---|---|---|
| egui + egui_glow + winit/glutin | Small explicit OSD surface, keyboard/drag/drop integration, application can control GL context and draw order | Initial choice; accessibility, text/input and redraw behavior need validation |
| iced + wgpu | Structured application state and custom shader primitives; useful for richer library UI | No automatic libmpv-to-wgpu native texture import; retain as alternative if a validated renderer bridge exists |
| GPUI | GPU UI with existing desktop applications | Greater framework/platform integration to evaluate; pre-1.0 changes and native media-surface bridge remain project work |
| Slint | Declarative UI and native integration options | Viable alternative; validate external GPU rendering hooks, toolkit version and chosen license before adoption |

[egui_glow](https://docs.rs/egui_glow/latest/egui_glow/) provides egui's GL renderer. [iced shader primitives](https://docs.iced.rs/iced/widget/struct.Shader.html) render with wgpu; they are not an established libmpv import API. [GPUI's README](https://github.com/zed-industries/zed/tree/main/crates/gpui) documents its development status. [Slint's license choices](https://github.com/slint-ui/slint/blob/master/LICENSE.md) must be assessed against the intended distribution.

## Decision

Use egui for interaction/layout, keeping presenter ownership outside a turnkey windowing framework. For the first portable render spike, render libmpv and egui sequentially in the same current OpenGL context and present once. Reset GL state between components. Use visible-control animations at refresh cadence; pause repainting when the controls/video do not change. Keep network/metadata/database operations off the window and render paths. Never stream CPU-decoded RGBA buffers through the UI image widget.

A Windows native D3D11 presenter will require a version-compatible D3D11 egui painting integration or a small painter for egui meshes. A switch to egui-wgpu does not solve D3D11/D3D12 resource sharing by itself. Treat that work as a separate measured spike. The portable GL choice remains provisional if macOS/Linux power results are unacceptable.

## Acceptance

Verify compositing/color/premultiplied alpha, HiDPI, resize/fullscreen and input; 60/120 Hz OSD interactions without missed video deadlines; paused/hidden-OSD idle power; screen reader/keyboard focus; platform IME; frame-boundary traces. Choose features/dependencies after inspecting release size and baseline power, not from toolkit marketing benchmarks.
