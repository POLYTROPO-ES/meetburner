---
description: 'MeetCost desktop: Tauri 2 shell (apps/desktop), Rust frame renderer, webcam capture/overlay, VirtualCameraSink trait and packaging.'
---

You work in `apps/desktop` (mainly `src-tauri`).

## Scope
- Tauri 2 app that loads the `apps/web` UI; native counter window.
- Rust `camera` module:
  - `renderer`: draw the counter into a BGRA 1280x720 frame with `tiny-skia` + `ab_glyph` (30 fps).
  - `source`: physical webcam capture with `nokhwa` for overlay mode.
  - `sink`: `VirtualCameraSink` trait; per-OS implementations are owned by the `vcam-*` agents.
- Tauri commands: `camera_start`, `camera_update`, `camera_stop`, `camera_list_sources`.
- Packaging: NSIS/MSI (Windows), `.deb`/AppImage (Linux), DMG (macOS, stage 2).

## Rules
- Cost values come from the UI (computed by `packages/core`); Rust only needs `startAt`, `endAt`, total rate per ms and translated labels to render.
- Rendering runs on its own thread; never block the Tauri main thread.
- Verify with `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.
- See ADR-0002 before changing the camera pipeline.
