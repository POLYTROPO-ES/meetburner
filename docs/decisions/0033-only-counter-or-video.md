# 0033. The camera frame is a choice: only counter, or video with a live preview

- Status: accepted
- Date: 2026-10-25
- Amends [ADR-0032](0032-one-counter-surface.md)

## Context

ADR-0032 derived the whole camera mode from the card rectangle: a card covering the frame
published the counter alone, anything smaller sat on a webcam. That reads well in code and badly
in the UI — the user could not tell "no webcam" from "webcam chosen but full-frame card", the
framing silently changed meaning when the card reached the edges, and the preview never showed
the video that the meeting app would receive.

The two things users actually decide are separate:

1. **Is there video behind the counter at all?** (meeting apps see the counter alone, or the
   counter over my webcam)
2. **Where and how big is the card?** (only meaningful when there is video)

## Decision

- `CameraSettings.video` stores the first decision. `false` publishes the counter alone and
  never opens the webcam, whatever camera is remembered; `true` composites the card over the
  chosen camera. Old settings without the field keep their meaning: a stored `source` meant
  video was on.
- The setup page offers it as a two-option switch over the preview: **Only counter** /
  **Add video background**. The camera controls (source, resolution, mirrors, driver) only
  appear in video mode, under the preview they affect.
- In video mode the preview shows the **live webcam behind the card**, captured with
  `getUserMedia` in the webview (permission granted by `on_permission_request`). It is a
  separate capture from the one the Rust side publishes, so any failure — denied permission, a
  camera another app holds, a device that disappears — only shows a message telling the user to
  pick another webcam or go back to only counter. The counter, the card and the virtual camera
  keep working: the setup preview can never take the app down.
- The video is mirrored in the preview when the source mirror flags are set, so the preview
  matches what the camera publishes.
- `effectiveOverlay` returns the whole frame whenever there is no video, so the Rust renderer
  keeps receiving `fullscreen` exactly as before; the wire format does not change.
- The style block moves beside the preview in a two-column workspace, with the preview column
  sticky, so the counter stays visible while its fonts, colours, background and rotation are
  edited. Below 900 px the columns stack and the preview is no longer sticky.

## Consequences

- One explicit switch replaces a rule nobody could see, and the camera options appear only when
  they matter.
- The preview needs a second webcam consumer. Drivers that allow one process at a time will show
  the error path — by design, that is the message the user asked for, not a broken page.
- The web build has no video mode at all: the switch, the camera options and the preview only
  exist where a virtual camera exists.
