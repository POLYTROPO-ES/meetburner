# 0035. The setup preview shows the webcam the camera pipeline captures

- Status: accepted
- Date: 2026-10-25
- Supersedes the preview capture of [ADR-0033](0033-only-counter-or-video.md)

## Context

ADR-0033 showed the chosen webcam behind the card by opening it a second time in the webview with
`getUserMedia`. On Windows that fails with `NotReadableError: Device in use` as soon as the virtual
camera is live: the Rust pipeline already holds the webcam to composite it, and most webcam drivers
(Media Foundation on Windows, UVC through V4L2 on Linux) accept a single consumer. The preview then
showed an error exactly when everything else worked.

## Decision

- **One consumer: the pipeline.** The webview never opens the webcam. The capture thread already
  keeps the newest frame; it now writes it, or the reason it has none, into a `Capture` shared with
  the engine.
- **`camera_preview_frame`** returns that frame shrunk to 480 px on its longer side as raw binary IPC:
  `[width u32 LE][height u32 LE]` then RGBA, empty while the webcam warms up, an error string when
  it could not be opened. No JPEG encoder is compiled in the dependency tree, and ~0.5 MB per frame
  is cheap locally.
- The setup page polls it about 12 times a second and draws into a `<canvas>` behind the card,
  mirrored like the output. States: *Starting the webcam…*, live, or the error with the advice to
  pick another webcam or switch to **Only counter**. The counter, the card and the virtual camera
  are never affected.
- With **Add video background** and a webcam chosen, the pipeline runs even before the driver is
  registered, so the preview always has a source. Leaving video mode without a driver stops it and
  releases the webcam.
- The webview no longer asks for camera permission: the `on_permission_request` grant is removed.

## Consequences

- The preview works whenever the virtual camera does, and shows the same frames it publishes.
- Opening failures (another app holds the webcam) reach the user with the system's own message.
- In video mode without a registered driver the pipeline renders frames nobody reads; that only
  lasts while the setup page is open in that mode.
- The preview is about 12 fps at 480 px: enough to place the card, not a monitor of the output.
