# 0037. Split-flap plates actually flip

- Status: accepted
- Date: 2026-10-25
- Supersedes the *No animation in this version* section of [ADR-0036](0036-split-flap-style.md)

## Context

ADR-0036 shipped the split-flap look as static plates, arguing that a web-only animation would
show something the virtual camera never sends. The reasoning about divergence was right; the
conclusion was not. The flip is the whole point of a mechanical board, and the camera is exactly
where it matters: people watch the counter inside a meeting.

Both surfaces can animate:

- the web counter already re-renders every animation frame and has the previous value in React
  state;
- the Rust renderer runs at 30 fps on its own thread and `RenderLoop` already lives across ticks,
  so it can keep the previous character and the moment it changed.

A flip lasts about 160 ms, i.e. five camera frames: enough to read as a mechanical fall, short
enough to settle between two updates of a running cost.

## Decision

### One model, two painters

The animation is defined once, as data, in `packages/core/src/themes.ts` and mirrored in Rust with
pinned tests (the same discipline as `platedWidth`, ADR-0036):

```ts
export const FLIP_MS = 160;      // one plate's fall
export const FLIP_SHADE = 0.35;  // how dark the falling flap gets

export interface FlapHalf {
  glyph: 'from' | 'to'; // which character this half shows
  scale: number;        // 1 = full half, 0 = flat on the seam
  shade: number;        // 0..FLIP_SHADE
}

export interface FlapFrame { top: FlapHalf; bottom: FlapHalf }

/** The two halves of a plate `progress` (0..1) through its flip. */
export function flapFrame(progress: number): FlapFrame;
```

A real plate falls in two steps, and so does this one:

| Progress | Top half | Bottom half |
|----------|----------|-------------|
| `0 .. 0.5` | the **old** glyph, compressed by `cos(π · progress)` towards the seam | still the **old** glyph, full size |
| `0.5 .. 1` | the **new** glyph, settled | the **old** glyph compressed by `cos(π · (progress − 0.5))` away from the seam |

The falling half is shaded by `FLIP_SHADE × (1 − scale)`, so it darkens as it turns.

### Which characters flip

Only characters whose value changed. An unchanged plate is not touched, so a still counter paints
exactly what ADR-0036 painted. If a character changes again while it is falling, the flip restarts
from the glyph currently on the plate: plates never queue, and the last value always wins.

The cost therefore flips the cent that changed (at 1,43 €/min about one plate every 0.4 s) and the
elapsed time flips one or two characters every second — the cadence of a board, not a blur.

### State

- Rust: `FlapState` (one entry per plated element and character: `from`, `to`, `started_ms`) is
  owned by `RenderLoop` and passed to `render`. An empty state paints settled plates, so every
  existing test and every non-flap theme stay byte-identical.
- Web: each plate remembers the character it showed and restarts a CSS animation when it changes.
  The hidden measurement rows (`.total.measure`) are rendered settled: they only exist to be
  measured.
- `prefers-reduced-motion: reduce` shows the new character immediately in the web; the camera has
  no such setting and keeps flipping.

### Painting

- Web: two half-faces per plate inside a `preserve-3d` wrapper. The old half rotates around the
  seam with `rotateX`, the new half sits behind it; `animation-duration: 160ms` comes from the
  shared constant through a CSS custom property.
- Rust: the plate is filled, the settled (new) half is drawn, then the falling half is rendered
  into a scratch pixmap of one plate and composited with a vertical scale around the seam, shaded.
  Glyphs stay clipped to the plate, as in ADR-0036.

## Consequences

- The web and the camera flip with the same model and duration, so the setup preview keeps telling
  the truth (ADR-0032, ADR-0035).
- Cost of a flip: one extra scratch composite per falling plate per frame, and only while it
  falls. At most a handful of plates are ever falling at once.
- Tests: core pins `flapFrame` at 0, ¼, ½, ¾ and 1, plus clamping; Rust pins that a changed
  character differs from the settled frame in the middle of the flip and matches it after
  `FLIP_MS`, and that an unchanged frame is not touched.
- A flip needs a clock: the web uses `performance.now()`, the camera the timestamp it already
  receives per tick, so no new payload field is needed.
- Reduced motion is honoured on the web; the camera is a video stream and has no such signal.
