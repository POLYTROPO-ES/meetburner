# Desktop (Tauri 2)

Wraps `apps/web`. Camera pipeline is planned in [roadmap](../../docs/plan/roadmap.md) M4–M8.

## First run

1. Install Rust and the Tauri prerequisites: https://v2.tauri.app/start/prerequisites/
2. Generate icons once (required by the Windows build): `npx tauri icon path/to/logo.png` (from this folder).
3. From the repo root: `npm run dev:desktop`.

## Camera pipeline (M4 + producer half of M6)

`src-tauri/src/camera/`:

| Module | Responsibility |
|--------|----------------|
| `frame` | BGRA 1280×720 frame, 4 bytes per pixel, no row padding |
| `theme` | theme ids, layouts, font choices; `"seasons"` resolved from the UTC month |
| `themes_data` | **generated** mirror of `packages/core/src/themes.ts` (ADR-0014) |
| `format` | `format_money`, `format_money_compact`, `format_duration` (mirror of `packages/core`) |
| `layout` | fullscreen/widget geometry and the `fitCost` rule |
| `fonts` | loads system fonts per `FontId` (no bundled font files) |
| `renderer` | draws the counter into a frame (tiny-skia + ab_glyph) |
| `ipc` | shared-memory frame protocol with a seqlock (ADR-0013) |
| `sink` | `VirtualCameraSink` trait, `NullSink`, `RecordingSink`, `WinSharedMemSink` |
| `engine` | 30 fps render thread, never on the Tauri main thread |

Tauri commands: `camera_start(config)`, `camera_update(config)`, `camera_stop()`.

The camera is live while the app is open: the UI starts it as soon as the driver is registered and stops it when the driver is cleaned (ADR-0018). Before the meeting starts the renderer publishes an idle frame (`00:00:00` / `0,00 €`), so meeting apps can preview the device. Camera errors are returned to the UI and shown in the setup page; they never stop the app.
`config` is the `CounterInput` shape (camelCase): `startAt`, `endAt`, `ratePerMs`, `currency`, `locale`, `labels{totalCost,elapsed,costPerMinute,startsIn}`, `display{theme,layout,costFont,timeFont,costScale,timeScale,overflow}`, `width`, `height`. Missing fields fall back to defaults and scales are clamped to 50–250 %.

- **Output size**: 16:9 (1920x1080, 1280x720, 640x360), 9:16 (1080x1920, 720x1280, 360x640), 4:3 (1440x1080, 960x720, 640x480), 3:4 (1080x1440, 720x960, 480x640) and 1:1 (1080x1080, 720x720, 480x480); default 1280x720 and anything else falls back to 720p. The setup page shows the selector in `fullscreen` layout only (ADR-0015).
- **Cost size**: the renderer fits the money to 90 % of the available width by default; `costScale` multiplies that fit (100 % = fill the width).
- **Live updates**: every edit calls `camera_update`, applied on the next frame (≤ 33 ms). A resolution change re-creates the shared mapping in the render thread; the app never stops.
- **Driver buttons**: the Camera section has **Register webcam for meets** (`camera_register_driver`, calls `DllRegisterServer` on the bundled `MeetBurnerCamera.dll`, per-user, no UAC) and **Clean webcam for meets** (`camera_unregister_driver`, deletes the HKCU keys itself, so it works even if the DLL is gone). ADR-0017.

On Windows, frames go to the mapping `Local\MeetBurnerCamera` (event `Local\MeetBurnerCameraFrame`); the DirectShow/MF DLLs read it. On other platforms the sink is a no-op until M5.

### Packaging the driver

`tools/copy-driver.mjs` runs before `tauri build` and again after it: it copies `native/vcam-windows/build/Release/MeetBurnerCamera.dll` into `src-tauri/resources/` (bundled into the NSIS/MSI installs via `bundle.resources`) and into `target/release/resources/` for the portable exe. Build the driver first:

```powershell
cmake -S native/vcam-windows -B native/vcam-windows/build -A x64
cmake --build native/vcam-windows/build --config Release
```

### Regenerate the theme mirror

After changing `packages/core/src/themes.ts`:

```powershell
npm run gen:themes
```

`npm test` fails if the committed `src-tauri/src/camera/themes_data.rs` is stale.

### Verify

```powershell
npm test
npm run build:web
cd apps/desktop/src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

