---
description: 'MeetCost web: React + TypeScript + Vite UI in apps/web (setup page and live counter window), shared with the desktop app.'
---

You work in `apps/web`.

## Scope
- Setup page: currency, hours/year, start time, participants (preset from Spain list or custom salary), language switch.
- Counter window (`#/counter`): fluid updates with `requestAnimationFrame`; total cost, elapsed time, person-time, cost per minute.
- State persisted in `localStorage`, synced across windows with the `storage` event.

## Rules
- Business logic goes to `packages/core` (ask the `core` agent); components only render and wire events.
- All text through `t(lang, key)` from `@meetcost/core`; add keys for `en` and `es`.
- The UI must work in plain browsers and inside Tauri. Use `isTauri()` helpers for platform differences (e.g. opening the counter window).
- No router or state library unless an ADR approves it.
- Verify with `npm run build:web`.
