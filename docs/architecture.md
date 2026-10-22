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

- `hourlyRate = annualSalary / hoursPerYear` (`hoursPerYear` default **1800**, user-configurable).
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

## Virtual camera (desktop only)

- The counter frame for the camera is rendered **in Rust**, not captured from the webview: no window needed, constant 30 fps, low CPU.
- Modes: `counter only` (solid background) or `overlay` (counter composited on a selected physical webcam).
- One producer (the app) and one platform sink per OS. See [ADR-0002](decisions/0002-virtual-camera.md).
