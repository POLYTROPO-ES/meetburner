# 0006. Counter styles, layouts and user sizing

- Status: accepted
- Date: 2026-10-22

## Context

The counter appears in a browser/desktop window and, on desktop, as a virtual webcam. Users want themes and control over the size and font of the cost and the time.

## Decision

- **Styles**: classic, modern, formal, halloween, christmas, 4 seasons, minimalist. "4 seasons" picks spring/summer/autumn/winter from the current date (northern hemisphere, meteorological seasons).
- Themes are **plain data** in `packages/core/src/themes.ts`: hex colours, a font id, weight, radius and an emoji decoration. The web UI maps them to CSS variables. The Rust camera renderer (M4) will read the same data, so both look the same.
- **Layouts**:
  - `fullscreen`: the whole frame/window is the counter. **Default for the camera when no source webcam is used.**
  - `widget`: a small card in the bottom-right corner. **Default for the camera when a source webcam is the background (overlay).**
  - In the web/desktop window the user can pick either layout (useful when sharing a screen).
- **User sizing**: separate font choice (`theme` default or one of system, serif, mono, rounded, condensed, handwritten, display) and size scale (50–250 %) for the cost and for the time values.
- Fonts are system font stacks only. No web fonts are downloaded (privacy, offline, no new dependencies).
- Display settings are stored on their own (`meetcost.display`) and checked with `normalizeDisplay`, because stored data isn't trusted.

## Consequences

- Text uses container query units (`cqw`/`cqh`), so the same component scales from the setup preview up to a full window.
- The Rust renderer has to bundle fonts that roughly match each font id, because it can't rely on CSS stacks.
