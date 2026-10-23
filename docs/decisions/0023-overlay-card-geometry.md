# 0023. The counter card can be moved and resized on the camera frame

- Status: accepted
- Date: 2026-10-23

## Context

ADR-0021 put the counter in a fixed bottom-right card: 42 % × 30 % of the frame with a 32 px margin.
That is a good default, but a meeting's own name tag, captions or a slide share often sit exactly
there, and there was no way to move the card without moving everyone else's UI.

## Decision

### Model

- `display.overlay` is an optional `{ x, y, w, h }` rectangle in **fractions of the frame** (0..1),
  shared by the web UI and the renderer. `null` keeps the built-in corner, so the stored payload of
  existing users stays valid and nothing changes until the card is touched.
- Core owns the geometry: `defaultOverlayRect(size)` returns the same numbers the renderer uses,
  `moveOverlay` stops at the frame edge without shrinking the card, `resizeOverlay` grows/shrinks
  from the bottom-right corner, and `clampOverlay` validates stored values (minimum 15 % of the
  frame, always inside).
- Rust validates again (`Overlay::sanitized`): non-finite numbers fall back to the built-in corner,
  and values are clamped, because the payload is untrusted.

### Editing

- A drag surface in the camera section (`OverlayEditor`): the card follows the pointer on a stage
  whose aspect ratio is the selected output size; the bottom-right handle resizes. Both gestures use
  the pure helpers above, so the maths is unit-tested and the component only maps pointer
  coordinates to fractions.
- The stage is a gesture surface, not a pixel-perfect preview: the renderer fits the font to the card
  (`fit_font_px`), which the editor approximates with container units.
- *Reset position* returns to the built-in corner (`null`).

### Rendering

`counter_layout_in(width, height, layout, overlay)` snaps the custom rectangle to whole pixels (the
same reason as ADR-0006: anti-aliased edges over video look dirty) and shrinks the card padding for
small cards so the content always has room.

## Consequences

- The card can sit anywhere, in any supported aspect ratio, and stays fully inside the frame.
- Fractions (not pixels) mean the same rectangle works for 640×360 and 1920×1080, including the
  portrait and square sizes.
- The web counter window does not use the rectangle: it is a window, not a camera frame.
