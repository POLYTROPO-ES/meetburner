# 0021. Webcam overlay: composite the counter over a real camera

- Status: accepted
- Date: 2026-10-22

## Context

Until M7 "MeetBurner Camera" only published the counter on a themed background. The usual case is
different: people want their own camera image with the cost in a corner, exactly like a caption. The
renderer already accepted a background frame (ADR-0006 defined the `widget` layout as "small corner
card over a source webcam"), but nothing captured that webcam.

## Decision

### Capture

- `camera/source.rs` adds `nokhwa` (features `input-native` + `decoding`). `decoding` is not optional:
  most webcams expose MJPEG at 1080p and without it the frame cannot be turned into RGB.
- Capture runs on its own thread (`meetburner-webcam`) and keeps only the newest frame in a mutex.
  The render thread never blocks on the camera: a slow device drops frames instead of stalling the
  virtual camera (the mapping keeps being written at 30 fps).
- Every frame is converted to BGRA and scaled to the output size, so the compositor always gets a
  frame that matches the payload.

### Compositing

- The payload carries `source` (`null` = counter only). Changing it, or the output size, restarts
  the capture; nothing else is torn down.
- The background is only used for the `widget` layout: the card is what makes the counter readable
  over video. With `fullscreen` the counter is drawn on the themed background as before, even if a
  source is configured, so a stray setting can never paint text over someone's face.
- The web UI switches the layout to `widget` when a webcam is selected and back to `fullscreen`
  when it is cleared.

### Fallback

- If the camera is busy, unplugged or missing, the source logs once and stays empty: the frame is
  the counter on the themed background and the virtual camera keeps publishing. Errors never stop
  the app (same rule as ADR-0018).
- "MeetBurner Camera" is not listed as a source: it is a DirectShow filter and `nokhwa` enumerates
  Media Foundation devices, so the overlay cannot feed itself.

## Consequences

- The camera section gains an *Overlay on webcam* selector, listing the physical cameras.
- CPU: capture and one nearest-neighbour scale per frame on the capture thread; the render thread
  still composites at 30 fps.
- Mirroring is not applied: the meeting app already mirrors your own preview, and the transmitted
  video is what other participants see. A mirror toggle can be added with the other overlay options
  (position, scale, opacity) in the rest of M7.
- A camera that only offers a format nokhwa cannot decode shows nothing and logs the reason; the
  counter keeps going.
