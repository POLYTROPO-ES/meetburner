# 0036. Split-flap style: values on airport plates

- Status: accepted
- Date: 2026-10-25

## Context

The seven presets (ADR-0006) and the per-style customisation (ADR-0029) change colours, fonts and
backgrounds, but every style still draws the counter as ordinary text. Users asked for the look of
the old airport boards — the split-flap (Solari) display — where each character sits on its own
plate with a horizontal seam across the middle.

Two constraints shape the decision:

- ADR-0014: anything visual has to exist as data in `themes.ts`, because the Rust camera renderer
  and the web UI must agree. A CSS-only trick would leave the camera looking different.
- ADR-0029: each style keeps its own tweaks. The new look must reuse that machinery instead of
  adding a second axis of settings.

## Decision

### A style, not a setting

`splitflap` joins `THEME_IDS` and `THEMES`. The look is a *style*, so it appears in the existing
style selector and inherits everything ADR-0029 already does: its own font/size/colour per element,
its own background, visibility and rotation, remembered per style.

`Theme` gains one field, so both renderers know how to draw it:

```ts
export type PlateStyle = 'none' | 'flap';

export interface Theme {
  // …unchanged fields…
  /** 'flap' draws values on split-flap plates; 'none' draws plain glyphs (ADR-0036). */
  plates: PlateStyle;
}
```

Every existing preset gets `plates: 'none'`, which must keep their output byte-identical. The
generated `themes_data.rs` carries the same field (`PlateStyle::None | PlateStyle::Flap`).

### The preset

| Token | Value | Why |
|-------|-------|-----|
| `background` | `#0b0c0f` | the board behind the plates, also the seam and the gap |
| `backgroundAlt` | `#1e2126` | the plate face |
| `text` | `#f7f3e8` | warm off-white digits |
| `muted` | `#a7a294` | captions stay quiet |
| `accent` | `#ffb020` | amber, like a running board |
| `font` | `condensed` | narrow bold face: the closest built-in stack to a Solari glyph |
| `weight` | `700` | |
| `radius` | `4` | the board itself has soft corners, the plates are nearly square |
| `decoration` | `''` | |

`styleDefaults` gives a split-flap style a **solid** background (everything else keeps the
gradient), because a gradient behind plates of a single flat colour reads as a mistake.

### One plate per character, on values only

- `PLATED_ELEMENTS` = `cost`, `elapsed`, `personTime`, `costPerMinute`. They are drawn one plate
  per character.
- `label` and `status` stay plain text: they are captions at 4 % of the frame height, and plating
  them would turn a caption into a strip of dots. This is one rule shared by both renderers
  (`isPlated(theme, element)` in core, the same check in Rust).
- Spaces advance without a plate. Every other character — digits, `:`, `.`, `,` and the currency
  symbol — gets its own plate, so `1.234,56 €` is eight characters on eight plates plus a space.
- Colours follow the style's background, so ADR-0029 tweaks keep working:
  - plate face = `BackgroundStyle.colorAlt` (preset: the theme's `backgroundAlt`),
  - gap and seam = `BackgroundStyle.color` (preset: the theme's `background`),
  - glyph = the element's colour, including the running accent.
  Plate face and seam use the background's opacity, so a translucent card shows the webcam
  through the plates exactly as it shows through the card.

### Geometry (shared numbers)

In ems of the element's font size, exported from `packages/core/src/themes.ts`:

| Constant | Value | Meaning |
|----------|-------|---------|
| `PLATE_WIDTH_EM` | `0.72` | width of one plate |
| `PLATE_GAP_EM` | `0.08` | gap between plates |
| `PLATE_HEIGHT_EM` | `1.15` | height of one plate |
| `PLATE_RADIUS_EM` | `0.08` | corner radius of a plate |
| `PLATE_SEAM_EM` | `0.04` | thickness of the seam line, at half the plate height |
| `PLATE_SPACE_EM` | `0.4` | advance of a space, which has no plate |

A non-space character advances `PLATE_WIDTH_EM + PLATE_GAP_EM`; the text width is the sum minus
the trailing gap, so the row is exactly as wide as it looks. `platedWidth(text, fontPx)` returns
that width, and the Rust renderer mirrors it — the cost fit (ADR-0015) then keeps working, because
the width no longer depends on glyph metrics.

Plates are taller than the glyph line (`1.15 em`), so the cost may need to be slightly smaller to
fit its box: the fit caps the font size at `box height / PLATE_HEIGHT_EM`, exactly as the web caps
it with the same constant.

### No animation in this version

A real board flips. The web could animate it with CSS, but the camera renders frames at 30 fps from
a stateless payload, so an animated web counter would show something the virtual camera never
sends. The style is therefore static in both renderers, and that is what makes it one look instead
of two. A flip animation is a separate decision.

### Accessibility

In the web the plate characters are `aria-hidden` and the value is also rendered once in an
`sr-only` span, so screen readers read `1.234,56 €`, not one character per plate. The camera has no
accessibility surface; nothing changes there.

## Consequences

- The style selector gains one entry (`theme.splitflap`, en/es) with no new UI: `THEME_IDS` drives
  the list, `gen:themes` carries the data to Rust, and `themes.rust.test.ts` fails if it is stale.
- The web needs per-character markup for plated values (`CounterView`), and its measurement
  elements must be plated too, or the fit would compare a plated width with a plain one.
- The Rust renderer needs `draw_plated` (rect, seam, centred glyph per cell) for the cost, the
  elapsed time and the cost per minute. The label keeps `draw_centered_tracked`.
- Plate colours come from the background, so painting the style in another palette means changing
  its background colour and alternate colour — no new controls.
- Follow-up, if asked for: a flip animation on the web counter only (new ADR).
