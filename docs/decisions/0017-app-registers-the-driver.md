# 0017. The app registers the camera driver

- Status: accepted
- Date: 2026-10-22

## Context

ADR-0016 made registration per-user, but it still required the user to build the DLL with CMake and run `regsvr32` (or the PowerShell scripts). Asking a meeting attendee to do that is not acceptable: registering or cleaning the virtual camera must be two buttons in the app.

## Decision

### Commands

| Command | What it does |
|---------|--------------|
| `camera_driver_status` | Reports `registered`, `available` (the DLL was found) and the DLL path in use |
| `camera_register_driver` | Locates the bundled DLL and calls its `DllRegisterServer` in-process (HKCU, no UAC) |
| `camera_unregister_driver` | Deletes the CLSID and video-input-category keys from `HKCU` directly |

- The **clean** path does not need the DLL: the app removes the registry trees itself with `RegDeleteTreeW`, so a missing or moved DLL cannot leave a broken camera behind.
- The **register** path keeps `DllRegisterServer` as the single source of truth for the key layout (the same export `regsvr32` and the installer use).

### Where the app looks for the DLL

In priority order:

1. `MEETBURNER_VCAM_DLL` (explicit override for testing)
2. `<exe dir>\MeetBurnerCamera.dll`
3. `<exe dir>\resources\MeetBurnerCamera.dll`
4. `<resource dir>\MeetBurnerCamera.dll` and `<resource dir>\resources\MeetBurnerCamera.dll`
5. `native/vcam-windows/build/Release/MeetBurnerCamera.dll` (development tree, for `tauri dev` / `cargo run`)

### Packaging

- `bundle.resources` includes the committed `resources/` directory, so `cargo build`/`clippy` stay valid even when the DLL has not been built yet (only a placeholder lives there in that case).
- `beforeBuildCommand` and `postbuild:desktop` run `tools/copy-driver.mjs`, which copies the DLL into `src-tauri/resources/` (bundled into the NSIS/MSI installs) and into `target/release/resources/` (so the portable exe works without installing).
- Packaging includes the driver when `native/vcam-windows` has been built; without it the app still builds and the buttons report "driver missing".

### UI

The **Camera** section shows two buttons — *Register webcam for meets* and *Clean webcam for meets* — plus the current state ("Registered …" / "Not registered yet …" / "driver missing"). Register is disabled while already registered or when the DLL is absent; clean is disabled while unregistered.

## Consequences

- End users never run scripts or `regsvr32`; one click registers "MeetBurner Camera" for their Windows account and one click removes every key.
- The installer still does not register on install; that is intentional (per-user, opt-in) and can be revisited in M9.
- The `native/vcam-windows/scripts/*.ps1` remain for CI, diagnostics and manual testing.
- A stale registration whose DLL was deleted can always be cleaned from the app.
