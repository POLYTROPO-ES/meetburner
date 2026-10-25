---
description: 'MeetBurner web: React + TypeScript + Vite UI in apps/web (setup page and live counter window), shared with the desktop app.'
---

You work in `apps/web`.

## Scope
- Setup page as a studio shell (ADR-0034): fixed header (live rate) and footer (totals, Reset, Share, Start/Stop), an editor column of collapsible `<details>` groups (Meeting, Participants, Counter style), a draggable splitter (`usePaneWidth`) and a preview pane that never leaves the screen.
- One counter surface (ADR-0032): the style selector, per-style customisation (fonts, sizes, colours, background, visibility, rotation; ADR-0029) and a single preview. On desktop the preview is the camera frame: `OverlayEditor` drags, resizes and turns the card, rendered with the same `CounterView` as the counter window. Layout is derived from the card, never chosen.
- **Only counter / Add video background** switch above the preview (`CameraSettings.video`, ADR-0033); camera options (resolution, mirrors, webcam, driver, MF status) in the panel under the preview (ADR-0034 amendment).
- Webcam preview: poll `camera_preview_frame` (~12 fps) and draw it into a `<canvas>`; the webview never opens the webcam with `getUserMedia` (ADR-0035).
- Counter window (`#/counter`): fluid updates with `requestAnimationFrame`; total cost, elapsed time, person-time, cost per minute. Share view (`#/share`, ADR-0009).
- State persisted in `localStorage`, synced across windows with the `storage` event; camera changes pushed with `camera_update` (`useCameraSync`, ADR-0015).
- Deployment: every push to `main` deploys to Cloudflare Pages through `.github/workflows/deploy-web.yml` (ADR-0012); security headers in `public/_headers`.

## Rules
- TDD (ADR-0008): business logic is first specified by a failing test in `packages/core`; UI-only pure helpers (platform detection, preview frame decoding, pane width) are tested with Vitest next to them in `apps/web/src/*.test.ts`.
- Business logic goes to `packages/core` (ask the `core` agent); components only render and wire events.
- All text through `t(lang, key)` from `@meetburner/core`; add keys for `en` and `es`.
- The UI must work in plain browsers and inside Tauri. Use `platform.ts`: `isTauri()` (a webview with a working `invoke`), `hasVirtualCamera()` (Windows and Linux desktop only; macOS and the web show no camera UI, ADR-0028, ADR-0030).
- Every camera command is invoked through `platform.ts`; a failed command shows a message and never breaks the page (ADR-0018).
- Target 1280×720 without page scroll; the workspace stacks below 900 px; keep keyboard and screen-reader support (separator, slider roles, live region).
- No router or state library unless an ADR approves it.
- Verify with `npm test` and `npm run build:web`; check layout changes with a screenshot of the running app.
