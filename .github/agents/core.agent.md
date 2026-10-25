---
description: 'MeetBurner core: pure TypeScript cost model, countries, salary presets, themes and en/es i18n in packages/core.'
---

You work only in `packages/core`.

## Scope
- Cost model (`cost.ts`): `hourlyRate = annualSalary × (1 + employerCostRate when total cost) / hoursPerYear`, snapshot computed from timestamps (`startAt`, `endAt`, `now`).
- Countries (`countries.ts`): currency, hours/year, employer contribution rate.
- Salary presets (`salaries.ts`): rough gross annual salaries per role and country (ADR-0005).
- Themes and display settings (`themes.ts`, ADR-0006): plain data shared with the Rust renderer.
  - Per-style customisation (`DisplaySettings.styles`, `ElementStyle`, `BackgroundStyle`, visibility, rotation; ADR-0029) with `normalizeDisplay` validating stored settings.
  - Card rectangle helpers (`OverlayRect`, move/resize/clamp, `fillsFrame`; ADR-0023). `Layout` is not stored: it is derived from the card (ADR-0032).
- Camera payload (`camera.ts`): `CameraSettings` (size, `source`, `video`; ADR-0015, ADR-0033), `effectiveOverlay` and `cameraInput`, which builds the exact `camera_start`/`camera_update` payload the Rust renderer reads.
- Share links (`share.ts`, ADR-0009); large amounts `fitCost` / `formatMoneyCompact` with short and long scale (ADR-0010, ADR-0011).
- i18n dictionary (`i18n.ts`) for `en` and `es`; every key must exist in both.
- Formatting helpers (money via `Intl.NumberFormat`, durations).

## Rules
- Any change to `THEMES` must be followed by `npm run gen:themes` (regenerates `apps/desktop/src-tauri/src/camera/themes_data.rs`); `themes.rust.test.ts` fails when it is stale (ADR-0014).
- Any change to `CameraInput` is a wire-format change: coordinate it with the `desktop` agent (Rust `CounterInput`) in the same PR.
- TDD (ADR-0008): write the failing Vitest test first, run `npm test` to see it red, then implement until green.
- No runtime dependencies, no DOM APIs: the code must run in browser, Node and be portable to Rust if needed.
- Every exported function has Vitest tests in `src/*.test.ts`. Run `npm test`.
- Keep the public API exported from `src/index.ts`.
