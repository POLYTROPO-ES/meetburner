---
description: 'MeetBurner desktop: Tauri 2 shell (apps/desktop), Rust frame renderer, webcam capture/overlay, VirtualCameraSink trait and packaging.'
---

You work in `apps/desktop` (mainly `src-tauri`).

## Scope
- Tauri 2 app that loads the `apps/web` UI; window opens at 1280×720 (minimum 960×600, ADR-0034); native counter window (M3, todo).
- Rust `camera` module:
  - `engine`: 30 fps render thread, live from app start while the driver is registered, idle frame before the meeting (ADR-0018); every `camera_update` applies on the next frame (ADR-0015).
  - `renderer` / `layout`: draw the counter into a BGRA frame of the selected size (640×360 up to 1920×1080, five ratios) with `tiny-skia` + `ab_glyph`; per-style fonts, colours, gradient, opacity, radius, visibility and rotation (ADR-0029); counter on its own layer, mirrored independently from the webcam (ADR-0022).
  - `themes_data.rs`: generated from `packages/core/src/themes.ts` by `npm run gen:themes`, never edited by hand; system fonts only (ADR-0014).
  - `source`: physical webcam capture with `nokhwa` on its own thread, newest frame only (ADR-0021); it also feeds the setup preview through `camera_preview_frame` (ADR-0035), so the webcam has a single consumer.
  - `driver` / `driver_embed`: register, clean and version-check the DirectShow driver (ADR-0017, ADR-0020, ADR-0026); `mf`: the Windows 11 camera calls (owned with `vcam-windows`).
  - `sink`: `VirtualCameraSink` trait; per-OS implementations are owned by the `vcam-*` agents. Platforms without a sink (macOS) use `NullSink` (ADR-0028).
- Tauri commands: `camera_start`, `camera_update`, `camera_stop`, `camera_preview_frame`, `camera_list_sources`, `camera_driver_status`, `camera_driver_ensure`, `camera_register_driver`, `camera_unregister_driver`, `camera_mf_status`, `camera_mf_register`, `camera_mf_unregister`, `camera_v4l2_status`.
- Packaging (with the `architect`, ADR-0025, ADR-0026, ADR-0028): Windows NSIS + MSI + portable one-file exe and zip, Linux `.deb` + AppImage, macOS universal `.dmg` (unsigned, no virtual camera). Version bump touches `apps/desktop/package.json`, `Cargo.toml`, `Cargo.lock`, `tauri.conf.json` and `package-lock.json`.

## Rules
- TDD (ADR-0008): write the failing `cargo test` first (structural frame tests, layout math), then implement. No PNG goldens: system fonts differ per machine (ADR-0014).
- Camera layout is not chosen by Rust: the payload's card rectangle decides (`fullscreen` when it covers the frame, `widget` otherwise), and `cameraInput` in `packages/core` sends the webcam only when `CameraSettings.video` is on (ADR-0032, ADR-0033).
- Cost values come from the UI (computed by `packages/core`); Rust receives `startAt`, `endAt`, total rate per ms, currency, locale, translated labels and the display settings, and computes the cost from timestamps on every frame.
- Keep frames byte-identical when a new option is at its default (e.g. rotation 0, all elements visible), so existing output never shifts.
- Rendering and capture run on their own threads; never block the Tauri main thread. Camera failures are reported to the UI, never fatal (ADR-0018).
- Verify with `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` (same as CI).
- See ADR-0002 before changing the camera pipeline, and ADR-0019 before touching frame orientation or the mapping.
