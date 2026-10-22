# 0010. Large amounts: abbreviate or shrink

- Status: accepted
- Date: 2026-10-22

## Context

Long meetings or high salaries produce amounts like `$208,421,052.63`. These overflow the counter, especially the small widget over a webcam.

## Decision

- A new display setting `overflow` lets the user choose:
  - `compact` (default): abbreviate with K, M, B, T, e.g. `$208.42M` or `12,35K €`.
  - `shrink`: keep the full amount and reduce the font.
- The web counter measures the rendered widths of the full and abbreviated texts in hidden copies. The pure `fitCost` function in core then picks the text and font scale.
- Available width: 90% of the counter in fullscreen and 50% in the widget, so the widget never covers most of the webcam image.
- If the abbreviated text is still too wide, compact mode also shrinks it.

## Consequences

- No dependencies. The rule is tested in core, and the camera renderer (M4) can reuse it with its own text measurements.
- The full amount is still available as a tooltip in the web counter.
