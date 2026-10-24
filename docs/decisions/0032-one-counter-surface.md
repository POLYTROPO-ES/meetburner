# 0032. One place to configure the counter

- Status: accepted
- Date: 2026-10-24

## Context

The setup page grew two configuration surfaces for the same card:

- **Display** had a preview of the counter window and the whole style block (fonts, colours,
  background, rotation, hidden lines);
- **Camera** had the drag/resize box (`OverlayEditor`, ADR-0023) that only painted the preset
  colours and two lines.

So the same card could be adjusted in two places that disagreed: rotating, hiding a line or
recolouring it changed the preview but not the box you use to place the card on the webcam, and
the box could not express what the camera actually sends. The `Layout` selector (fullscreen /
widget) was a third, redundant control: it was already set from whether a webcam source was
chosen, and "fullscreen" is just a card that covers the frame.

The web app has no virtual camera at all, so every camera-only control there is noise.

## Decision

**One section, one preview, one style block.**

- The **Counter** section holds the style selector, the preview, the style block and the card
  actions. The **Camera** section keeps only what is about the camera: source, resolution, the
  four mirror flags and the driver actions.
- The preview *is* the camera frame when a virtual camera exists (desktop): a card that can be
  dragged, resized and turned, rendered with the live style so it always matches the output. On
  the web the same section shows the plain counter, without handles.
- **`Layout` disappears from `DisplaySettings`.** `cameraInput` derives it from the card:
  a rectangle that covers the frame renders with the renderer's fullscreen geometry, anything
  smaller renders as a card (`widget`). A **Fill frame** button writes the full-frame rectangle
  and **Reset position** returns to the built-in corner. The Rust wire format is unchanged, so
  the renderer keeps both geometries.
- The card rectangle stays **global** (`display.overlay`): where the card sits belongs to the
  camera frame, not to a style. Everything that changes the look stays per style (ADR-0029).
- The overflow selector (abbreviate or shrink long amounts) moves next to the other style
  controls and is shown on the web too, because it shapes both the camera and the counter
  window. It stays one global setting: it is a formatting preference, not part of a look.
- Rotation gets a **rotate handle** on the card in addition to the slider, so the gesture and
  the keyboard path both exist (the handle is a `slider` role, arrows turn the card).

## Consequences

- One picture to trust: nothing can drift between a preview and the camera.
- The counter window can no longer be set to the small "widget" presentation. That presentation
  only ever made sense as the camera overlay, and the camera owns it now.
- Stored settings that carry `layout` or a top-level `overflow` are ignored and fall back to the
  derived values; `normalizeDisplay` keeps validating everything else.
- The stage renders the same `CounterView` as the counter window instead of its own two-line
  card, so the style block and the card cannot disagree again. The stage remains a gesture
  surface, not a pixel-perfect render of the Rust renderer (ADR-0023).
