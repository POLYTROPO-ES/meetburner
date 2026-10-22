---
description: 'MeetBurner core: pure TypeScript cost model, countries, salary presets, themes and en/es i18n in packages/core.'
---

You work only in `packages/core`.

## Scope
- Cost model: `hourlyRate = annualSalary × (1 + employerCostRate when total cost) / hoursPerYear`, snapshot computed from timestamps (`startAt`, `endAt`, `now`).
- Countries (`countries.ts`): currency, hours/year, employer contribution rate.
- Salary presets (`salaries.ts`): rough gross annual salaries per role and country (ADR-0005).
- Themes and display settings (`themes.ts`, ADR-0006): plain data shared with the Rust renderer.
- i18n dictionary (`i18n.ts`) for `en` and `es`; every key must exist in both.
- Formatting helpers (money via `Intl.NumberFormat`, durations).

## Rules
- TDD (ADR-0008): write the failing Vitest test first, run `npm test` to see it red, then implement until green.
- No runtime dependencies, no DOM APIs: the code must run in browser, Node and be portable to Rust if needed.
- Every exported function has Vitest tests in `src/*.test.ts`. Run `npm test`.
- Keep the public API exported from `src/index.ts`.
