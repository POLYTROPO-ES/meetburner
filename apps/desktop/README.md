# Desktop (Tauri 2)

Wraps `apps/web`. Camera pipeline is planned in [roadmap](../../docs/plan/roadmap.md) M4–M8.
End-user instructions live in the [user guide](../../docs/user-guide.md).

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
| `layout` | fullscreen/widget geometry and the `fitCost` rule; the mode is derived from the card rectangle (ADR-0032) |
| `fonts` | loads system fonts per `FontId` (no bundled font files) |
| `renderer` | draws the counter into a frame (tiny-skia + ab_glyph) |
| `ipc` | shared-memory frame protocol with a seqlock (ADR-0013) |
| `sink` | `VirtualCameraSink` trait, `NullSink`, `RecordingSink`, `WinSharedMemSink` |
| `engine` | 30 fps render thread, never on the Tauri main thread |

Tauri commands: `camera_start(config)`, `camera_update(config)`, `camera_stop()`.

The camera is live while the app is open: the UI starts it as soon as the driver is registered and stops it when the driver is cleaned (ADR-0018). Before the meeting starts the renderer publishes an idle frame (`00:00:00` / `0,00 €`), so meeting apps can preview the device. Camera errors are returned to the UI and shown in the setup page; they never stop the app.

On start the app also checks the driver version (`camera_driver_ensure`) and re-registers an older installation with the bundled DLL (ADR-0020).

Overlay mode (M7, ADR-0021): `camera/source.rs` captures a physical webcam with `nokhwa` on its own thread; the payload's `source` selects it and the derived `widget` layout composites the card over the video ([ADR-0032](../../docs/decisions/0032-one-counter-surface.md)). `MEETBURNER_SOURCE=<id>` on `produce_frames` exercises the same path without the UI.

The card can be moved, resized and turned from the setup page (`display.overlay`, ADR-0023, ADR-0032); `MEETBURNER_OVERLAY=x,y,w,h` does the same for placement from `produce_frames`.
`config` is the `CounterInput` shape (camelCase): `startAt`, `endAt`, `ratePerMs`, `currency`, `locale`, `labels{totalCost,elapsed,costPerMinute,startsIn}`, `display{theme,layout,overflow,mirror{sourceH,sourceV,overlayH,overlayV},overlay{x,y,w,h},rotation,elements,background}`, `width`, `height`, `source`. Missing fields fall back to defaults and scales are clamped to 50–250 %.

- **Output size**: 16:9 (1920x1080, 1280x720, 640x360), 9:16 (1080x1920, 720x1280, 360x640), 4:3 (1440x1080, 960x720, 640x480), 3:4 (1080x1440, 720x960, 480x640) and 1:1 (1080x1080, 720x720, 480x480); default 1280x720 and anything else falls back to 720p (ADR-0015).
- **Cost size**: the renderer fits the money to 90 % of the available width by default; `costScale` multiplies that fit (100 % = fill the width).
- **Live updates**: every edit calls `camera_update`, applied on the next frame (≤ 33 ms). A resolution change only rewrites the frame header: the shared mapping is created once and never re-created (ADR-0019).
- **Driver buttons**: the Camera section has **Register webcam for meets** (`camera_register_driver`, calls `DllRegisterServer` on the driver bundled with the app, per-user, no UAC) and **Clean webcam for meets** (`camera_unregister_driver`, deletes the HKCU keys itself, so it works even if the DLL is gone). ADR-0017.

On Windows, frames go to the mapping `Local\MeetBurnerCamera` (event `Local\MeetBurnerCameraFrame`); the DirectShow/MF DLLs read it. On other platforms the sink is a no-op until M5.

### Packaging the driver

`tools/copy-driver.mjs` runs before `tauri build` and copies `native/vcam-windows/build/Release/MeetBurnerCamera.dll` into `src-tauri/resources/`. `build.rs` embeds that real DLL in the exe (ADR-0026), so the portable build is a single file; the same folder is bundled into the NSIS/MSI installs via `bundle.resources`. A committed placeholder or a missing DLL is not embedded, so `cargo run` and `tauri dev` keep loading the CMake build. Build the driver first:

```powershell
cmake -S native/vcam-windows -B native/vcam-windows/build -A x64
cmake --build native/vcam-windows/build --config Release
```

## Install vs portable

```powershell
npm run build:desktop   # NSIS + MSI in src-tauri/target/release/bundle
```

| | Installer | Portable, one file | Portable, zip |
|---|---|---|---|
| Files | `MeetBurner_<version>_x64-setup.exe` or `..._en-US.msi` | `MeetBurner_<version>_x64.exe` | the same exe + `resources\MeetBurnerCamera.dll` |
| Web UI | embedded in the exe (`frontendDist`) | embedded | embedded |
| Camera driver | copied by the installer, registered on demand | embedded in the exe; the first *Register webcam for meets* writes `MeetBurnerCamera.v{N}.dll` next to it (ADR-0026) | already on disk, registered as is; nothing is extracted |
| Update | run the newer installer | replace the single file | replace both files |
| Settings | WebView2 profile of the app id | same (not next to the exe) | same |

### Deploying the portable build

Both shapes contain the same `meetburner.exe`:

- **One file**: nothing else to copy, and the driver appears beside the exe the first time the camera is registered.
- **Zip**: the exe plus `resources\MeetBurnerCamera.dll`. Prefer it when antivirus or SmartScreen flags the single file: an unsigned exe that writes a DLL and registers a COM server looks like a dropper to heuristics, while the zip extracts and writes nothing.

1. Copy the exe (and `resources\` when using the zip) to a writable folder (the Desktop or a USB stick works; no admin needed).
2. Run the exe. Windows SmartScreen warns because it is not signed yet: *More info → Run anyway*.
3. Open *Camera* and press **Register webcam for meets** once per Windows account (HKCU, no UAC). With the single file, the first click writes the embedded driver as `MeetBurnerCamera.v{N}.dll` beside the exe and registers that copy; with the zip it registers the DLL from `resources\`. Do the same on every machine where the stick is used.
4. The single-file build needs a writable folder. When MeetBurner cannot write the driver (for example `C:\Program Files` without elevation, or a write-protected USB stick), the camera section reports *the folder that holds MeetBurner.exe is read-only; move the app to a writable folder to register the camera*.
5. To update, close the app and replace the file(s). The next start extracts the driver of the new build and, when the camera is already registered, re-registers it if it is newer (ADR-0020).
6. To remove it, press **Clean webcam for meets** first (it deletes the registry keys even if the DLL is gone), then delete the exe and, for the single-file build, the extracted `MeetBurnerCamera.v{N}.dll`.

What each file is for:

- `meetburner.exe` — the app, the Rust camera pipeline and the web UI, plus the DirectShow driver in the single-file build; nothing else is needed to run the counter.
- `resources\MeetBurnerCamera.dll` — the driver as packaged by the zip and the installer; it is registered as is.
- `MeetBurnerCamera.v{N}.dll` — written next to the exe on the first registration of the single-file build, because `InprocServer32` needs a file path. It is version-checked on start and re-registered when the bundled driver is newer (ADR-0020). Meeting apps may keep it loaded, so a file with the same length is reused instead of rewritten. Development builds without a native driver keep loading the CMake build from `native/vcam-windows/build/Release`.

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

