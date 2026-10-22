# 0014. Rust renderer: system fonts and generated theme data

- Status: accepted
- Date: 2026-10-22

## Context

[ADR-0006](0006-counter-styles-and-layouts.md) decided that the camera renderer must look like the web counter: same themes, same layout rules, same overflow behaviour. Two gaps had to be closed before implementing it:

1. **Fonts.** The web uses CSS font stacks; `ab_glyph` needs actual font files.
2. **Theme data.** The themes live in TypeScript (`packages/core/src/themes.ts`) and the renderer is Rust.

## Decision

### Fonts: read them from the OS

- The Rust renderer loads system fonts at startup from the usual per-OS directories (`%WINDIR%\Fonts` and the per-user font folder on Windows, `/usr/share/fonts/...` on Linux, `/System/Library/Fonts` on macOS).
- Each `FontId` has an ordered candidate list (Segoe UI, Georgia, Consolas, Segoe Print, Impact… with DejaVu/Liberation fallbacks); the first file that parses wins.
- If an id has no usable file, it falls back to the first font that loaded. If nothing loads, `camera_start` fails with a readable error.
- No font binaries are added to the repository: no size, no licence files to audit, no installers to update.

### Theme data: generate Rust from TypeScript

- `packages/core/src/themes.rust.ts` renders `themes.rs` from the shared `THEMES` object; `npm run gen:themes` writes `apps/desktop/src-tauri/src/camera/themes_data.rs`.
- The generated file is committed and marked `#[rustfmt::skip]`, so formatting tools never create diffs against the generator.
- A Vitest test (`themes.rust.test.ts`) fails when the committed file is stale, so the web UI and the camera cannot drift apart silently.
- The renderer resolves `"seasons"` at frame time from the current UTC month, exactly like `resolveTheme`.

### Drawing

- Text is rasterised with `ab_glyph` into a `tiny-skia` pixmap (`tiny-skia` also draws the widget card), then converted to the BGRA frame.
- Glyphs are clipped to their text box. This guarantees that overlay mode never paints a pixel outside the widget card, even when a value is wider than expected.
- Theme emoji decorations are only drawn when the chosen face actually contains the glyph; otherwise they are skipped instead of rendering tofu boxes.

## Consequences

- Rendering is identical on a given machine, but not bit-identical across Windows builds that ship different font versions. Tests therefore assert structure (dimensions, theme colours, ink inside each text box, zero ink outside the widget card) instead of comparing PNG goldens.
- If we ever need pixel-exact goldens or emoji decorations, we bundle a few OFL fonts and supersede this ADR.
- Adding a theme is one edit in `themes.ts` plus `npm run gen:themes`; forgetting the second step fails `npm test`.
