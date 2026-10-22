# 0015. Camera output size, width-filling cost and live updates

- Status: accepted
- Date: 2026-10-22

## Context

Three desktop requests shaped the camera after the first renderer release:

1. In fullscreen mode the user must be able to choose the camera resolution.
2. The cost must dominate the frame: its font size should cover most of the width by default.
3. Any parameter change in the app must reach the virtual camera immediately, without stopping or pausing anything.

## Decision

### Output size

- `width` and `height` are part of the camera payload (`CounterInput` in Rust, `CameraInput` in `packages/core/src/camera.ts`).
- Allowed sizes: **640×360**, **1280×720** and **1920×1080**. Anything else falls back to 1280×720, so untrusted persisted settings can never ask for a 4K 60 fps stream.
- The resolution selector is shown for the `fullscreen` layout only. Overlay mode (M7) will inherit the source webcam size instead.
- The list lives in both `packages/core/src/camera.ts` and `apps/desktop/src-tauri/src/camera/frame.rs`; the Rust side is the enforcement point, so a stale UI list can only degrade to 720p.

### Cost font fills the width

- `fit_font_px` measures the money text and returns the em size whose advance width is **90 % of the available box width**, capped at the box height (money ink is about 0.7 em tall, so a full-height em size still fits).
- `costScale` (the existing 50–250 % slider) multiplies that fitted size: 100 % now means "fill the width".
- The existing compact/shrink rule still applies when the user enlarges the cost past the box.

### Live updates

- Every UI edit calls `camera_update`; the render thread reads the shared input on every tick, so text, theme, fonts, scale, labels and currency change within one frame (≤ 33 ms).
- A resolution change is the only operation that touches the sink: the render thread calls `stop()` and `start()` with the new size, which closes and re-creates `Local\MeetBurnerCamera` with the new mapping length. The app and the render thread never stop.
- Consumers keep their old view until they notice the stale timestamp (ADR-0013) and re-open the mapping; they show their "not running" frame in between.

## Consequences

- No restart, no pause, no re-selection of the camera in the meeting app for normal edits.
- A resolution change can blink for the consumer's stale timeout (about 1 s) while the mapping is re-created. Acceptable: users change resolution before a meeting, not during one.
- The web counter window keeps its CSS sizing; the fill rule applies to the camera feed rendered in Rust.
- Adding a size means touching the two lists above and the Rust tests.
