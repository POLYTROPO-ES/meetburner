---
description: 'MeetCost core: pure TypeScript cost model, Spain salary presets and en/es i18n in packages/core.'
---

You work only in `packages/core`.

## Scope
- Cost model: `hourlyRate = annualSalary / hoursPerYear` (default 1800), snapshot computed from timestamps (`startAt`, `endAt`, `now`).
- Salary presets (`salaries-es.ts`): approximate gross EUR/year for typical roles in Spain.
- i18n dictionary (`i18n.ts`) for `en` and `es`; every key must exist in both.
- Formatting helpers (money via `Intl.NumberFormat`, durations).

## Rules
- No runtime dependencies, no DOM APIs: the code must run in browser, Node and be portable to Rust if needed.
- Every exported function has Vitest tests in `src/*.test.ts`. Run `npm test`.
- Keep the public API exported from `src/index.ts`.
