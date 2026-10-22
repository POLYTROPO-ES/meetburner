# Architecture

## Overview

```mermaid
flowchart LR
  subgraph Shared
    core["packages/core<br/>cost model, presets, i18n"]
    ui["apps/web<br/>React UI (setup + counter)"]
  end
  core --> ui
  ui -->|static build| web["Web (browser)"]
  ui -->|frontend| tauri["apps/desktop (Tauri 2)"]
  subgraph Rust["src-tauri (Rust)"]
    renderer["Frame renderer<br/>tiny-skia + ab_glyph"]
    capture["Source webcam capture<br/>nokhwa"]
    sink["VirtualCameraSink trait"]
  end
  tauri --> renderer
  capture --> renderer
  renderer --> sink
  sink -->|shared memory| win["native/vcam-windows<br/>DirectShow + MF"]
  sink -->|/dev/videoN| lin["v4l2loopback"]
  sink -->|CMIO sink stream| mac["native/vcam-macos (stage 2)"]
```

## Cost model (packages/core)

- A meeting has a country (ES, PT, FR, DE, IT, NL, GB, US, MX) that sets default currency, hours/year and employer contribution rate ([ADR-0005](decisions/0005-countries-and-total-cost.md)).
- `hourlyRate = annualSalary × (1 + employerCostRate if total cost) / hoursPerYear` (Spain: 1800 h, all user-configurable).
- `totalCost(t) = Σ hourlyRate_i × elapsedHours(t)`.
- `personTime(t) = participants × elapsed(t)`.
- `elapsed(t) = clamp(min(now, endAt) − startAt, 0)`. A start time in the future shows a "starts in" state.
- Counter is computed from timestamps (never accumulated), so every window/frame is consistent and drift-free.

## Web flow

1. Setup page: currency, hours/year, start time, participants (preset role from Spain salary list or custom annual salary).
2. **Start** stores the meeting in `localStorage` and opens the counter window (`#/counter`):
   - Web: `window.open` popup.
   - Desktop: Tauri `WebviewWindow`.
3. Counter window re-renders every animation frame (`requestAnimationFrame`) and listens to `storage` events for live edits.
4. **Share the cost** builds a `#/share?t=…&c=…&m=…&cur=…` link carrying only the aggregate figures (total time, total cost, cost/min) and copies it to the clipboard. `ShareView` decodes and validates it ([ADR-0009](decisions/0009-share-links.md)).

## Counter styles ([ADR-0006](decisions/0006-counter-styles-and-layouts.md))

- Themes (classic, modern, formal, halloween, christmas, 4 seasons, minimalist) are plain data in `packages/core/src/themes.ts`, shared by the web UI (CSS variables) and the Rust camera renderer.
- Layout `fullscreen` (camera without source webcam) or `widget` (small card over a source webcam).
- User picks the font and size of the cost and of the time. `CounterView` renders the setup preview and the counter window.

## Hosting

Web app: static build on Cloudflare Pages with security headers from `apps/web/public/_headers` ([ADR-0007](decisions/0007-cloudflare-hosting.md)).

## Virtual camera (desktop only)

- The counter frame for the camera is rendered **in Rust**, not captured from the webview: no window needed, constant 30 fps, low CPU.
- Modes: `counter only` (fullscreen layout, solid themed background) or `overlay` (widget layout composited on a selected physical webcam).
- One producer (the app) and one platform sink per OS. See [ADR-0002](decisions/0002-virtual-camera.md).

### Implemented pipeline (M4 + producer of M6)

`apps/desktop/src-tauri/src/camera/`:

```
camera_start/update/stop (Tauri commands)
        │  CounterInput (startAt, endAt, ratePerMs, currency, locale, labels, display)
        ▼
   engine (30 fps thread) ──► renderer (tiny-skia + ab_glyph) ──► Frame (BGRA 1280x720)
        │                                                              │
        │  themes_data.rs (generated from themes.ts, ADR-0014)         ▼
        └─ fonts (system font files, no bundled binaries)      VirtualCameraSink
                                                                      │
                                          Windows: WinSharedMemSink ──┘
                                          Local\MeetBurnerCamera + event (ADR-0013)
                                                                      │
                                                          DirectShow / MF DLL (M6/M8)
```

- The cost is computed from timestamps on every frame (`ratePerMs × elapsed`), never accumulated, so the camera and the windows always agree.
- The cost font is fitted to 90 % of the available width by default; the user size multiplies that fit, and the compact/shrink rule applies past the box.
- The output size is part of the camera payload (`640×360`, `1280×720`, `1920×1080`); the setup page offers it in fullscreen mode. Unknown sizes fall back to 720p.
- `camera_update` pushes every parameter change to the render thread, which applies it on the next frame. A resolution change closes and re-creates the shared mapping inside the render thread; nothing stops (ADR-0015).
- Overlay mode keeps the webcam frame as the background and draws the themed card (`widget`) in the bottom-right corner; glyphs are clipped to their box so nothing is painted over the webcam.
- The Windows producer writes a 48-byte header (magic, version, size, fps, state, frame counter, timestamp, seqlock) followed by the BGRA payload into a named shared-memory mapping. Consumers read it with the seqlock retry loop, so they never block the producer and never get a torn frame.
