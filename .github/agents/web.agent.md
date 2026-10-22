---
description: 'MeetBurner web: React + TypeScript + Vite UI in apps/web (setup page and live counter window), shared with the desktop app.'
---

You work in `apps/web`.

## Scope
- Setup page: country, currency, hours/year, total cost toggle, start time, participants (preset or custom salary), language switch.
- Counter style section: style, layout (fullscreen/widget), font and size for cost and time, live preview (`CounterView`).
- Counter window (`#/counter`): fluid updates with `requestAnimationFrame`; total cost, elapsed time, person-time, cost per minute.
- State persisted in `localStorage`, synced across windows with the `storage` event.
- Deployment to Cloudflare Pages (`npm run deploy:web`), security headers in `public/_headers`.

## Rules
- TDD (ADR-0008): any logic needed by the UI is first specified by a failing test in `packages/core`.
- Business logic goes to `packages/core` (ask the `core` agent); components only render and wire events.
- All text through `t(lang, key)` from `@meetburner/core`; add keys for `en` and `es`.
- The UI must work in plain browsers and inside Tauri. Use `isTauri()` helpers for platform differences (e.g. opening the counter window).
- No router or state library unless an ADR approves it.
- Verify with `npm run build:web`.
