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

On start the app also checks the driver version (`camera_driver_ensure`) and re-registers an older installation with the bundled DLL (ADR-0020).

Overlay mode (M7, ADR-0021): `camera/source.rs` captures a physical webcam with `nokhwa` on its own thread; the payload's `source` selects it and the `widget` layout composites the card over the video. `MEETBURNER_SOURCE=<id>` on `produce_frames` exercises the same path without the UI.

The card can be moved and resized from the setup page (`display.overlay`, ADR-0023); `MEETBURNER_OVERLAY=x,y,w,h` does the same from `produce_frames`.
`config` is the `CounterInput` shape (camelCase): `startAt`, `endAt`, `ratePerMs`, `currency`, `locale`, `labels{totalCost,elapsed,costPerMinute,startsIn}`, `display{theme,layout,costFont,timeFont,costScale,timeScale,overflow,mirror{sourceH,sourceV,overlayH,overlayV},overlay{x,y,w,h}}`, `width`, `height`, `source`. Missing fields fall back to defaults and scales are clamped to 50–250 %.

- **Output size**: 16:9 (1920x1080, 1280x720, 640x360), 9:16 (1080x1920, 720x1280, 360x640), 4:3 (1440x1080, 960x720, 640x480), 3:4 (1080x1440, 720x960, 480x640) and 1:1 (1080x1080, 720x720, 480x480); default 1280x720 and anything else falls back to 720p (ADR-0015).
- **Cost size**: the renderer fits the money to 90 % of the available width by default; `costScale` multiplies that fit (100 % = fill the width).
- **Live updates**: every edit calls `camera_update`, applied on the next frame (≤ 33 ms). A resolution change only rewrites the frame header: the shared mapping is created once and never re-created (ADR-0019).
- **Driver buttons**: the Camera section has **Register webcam for meets** (`camera_register_driver`, calls `DllRegisterServer` on the bundled `MeetBurnerCamera.dll`, per-user, no UAC) and **Clean webcam for meets** (`camera_unregister_driver`, deletes the HKCU keys itself, so it works even if the DLL is gone). ADR-0017.

On Windows, frames go to the mapping `Local\MeetBurnerCamera` (event `Local\MeetBurnerCameraFrame`); the DirectShow/MF DLLs read it. On other platforms the sink is a no-op until M5.

### Packaging the driver

`tools/copy-driver.mjs` runs before `tauri build` and again after it: it copies `native/vcam-windows/build/Release/MeetBurnerCamera.dll` into `src-tauri/resources/` (bundled into the NSIS/MSI installs via `bundle.resources`) and into `target/release/resources/` for the portable exe. Build the driver first:

```powershell
cmake -S native/vcam-windows -B native/vcam-windows/build -A x64
cmake --build native/vcam-windows/build --config Release
```

## Install vs portable

```powershell
npm run build:desktop   # NSIS + MSI in src-tauri/target/release/bundle
```

| | Installer | Portable |
|---|---|---|
| Files | `MeetBurner_<version>_x64-setup.exe` or `..._en-US.msi` | `meetburner.exe` + `resources\MeetBurnerCamera.dll` |
| Web UI | embedded in the exe (`frontendDist`) | embedded in the exe |
| Camera driver | copied by the installer, registered on demand | same, loaded from the copy beside the exe |
| Update | run the newer installer | replace the two files |
| Settings | WebView2 profile of the app id | same (not next to the exe) |

### Deploying the portable build

1. Copy `meetburner.exe` and `resources\MeetBurnerCamera.dll` to any folder (a USB stick works; no admin needed).
2. Run `meetburner.exe`. Windows SmartScreen warns because the exe is not signed yet: *More info → Run anyway*.
3. Open *Camera* and press **Register webcam for meets** once per Windows account (HKCU, no UAC). Do the same on every machine where the stick is used.
4. To update, close the app and replace both files. To remove it, press **Clean webcam for meets** first (it deletes the registry keys even if the DLL is gone), then delete the folder.

What each file is for:

- `meetburner.exe` — the app, the Rust camera pipeline and the web UI; nothing else is needed to run the counter.
- `resources\MeetBurnerCamera.dll` — the DirectShow driver the register button loads; it is version-checked on start and re-registered when the bundled one is newer (ADR-0020). Without it the app still runs and the counter works, but the camera section reports *the camera driver is missing next to the app*.
- Anything else inside `resources\` is a leftover of `bundle.resources` and is not used at runtime.

Settings, meeting configuration and the camera layout live in the WebView2 profile (`%LOCALAPPDATA%\app.meetburner.desktop`), so they follow the Windows account rather than the folder; copying the folder does not copy them.

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

