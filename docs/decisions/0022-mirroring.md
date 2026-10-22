# 0022. Mirroring: the webcam and the counter are flipped independently

- Status: accepted
- Date: 2026-10-22

## Context

The counter reached Edge/Meet flipped. Meeting apps mirror the local preview of your own camera, and
depending on the capture stack the frame itself can also arrive flipped (ADR-0019 fixed the row order
for Media Foundation, but not every consumer reads the DIB the same way). There is no single
orientation that satisfies all of them, and the two layers have opposite needs:

- **the webcam** is usually better mirrored (it behaves like a mirror), which is what apps do for their
  own preview;
- **the counter** must always be readable: mirroring it turns the numbers into glyph soup.

## Decision

`display.mirror` carries four independent flags, all `false` by default:

| Flag | Effect |
|------|--------|
| `sourceH` / `sourceV` | flips the captured webcam before it is composited |
| `overlayH` / `overlayV` | flips the counter layer (card and text together) |

- The renderer draws the counter on its **own transparent pixmap** and composites it over the base
  frame at the end, so the two layers can be flipped separately (`flip_pixmap`).
- The webcam flip is applied to the captured `Frame` (`Frame::flipped`), after it has been scaled to
  the output size, so the scale is never mirrored twice.
- The setup page exposes them as two groups of two checkboxes ("Camera image" and "Counter card"),
  desktop only; the web counter window is unaffected, because it is not a camera.
- With no webcam (fullscreen) the only meaningful flags are the overlay ones; they work there too.

## Consequences

- A user whose meeting app mirrors its own preview can now compensate without touching the counter.
- A wrong orientation is a setting, not a code change: no more guessing at what each consumer does
  with `biHeight`.
- The composite adds one extra full-size pixmap per frame (and a copy when the overlay is flipped).
  At 1280×720 / 30 fps that is ~3.7 MB per frame, reuse-free but short-lived, and the render thread
  measures the same as before in practice.
- Text drawn on a transparent layer and blended over video has slightly softer edges than when it was
  drawn directly; the per-theme ink tests still pass unchanged.
