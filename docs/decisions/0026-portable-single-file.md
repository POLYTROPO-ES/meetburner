# 0026. The portable build ships as one file and as a folder

- Status: accepted
- Date: 2026-10-23
- Related: [0025](0025-github-releases.md) (release pipeline), [0017](0017-app-registers-the-driver.md) (the app bundles the driver), [0020](0020-driver-version-and-self-update.md) (driver version check)

## Context

The portable download used to be a zip with `meetburner.exe` and `resources\MeetBurnerCamera.dll`. One file is nicer to hand out, but only the virtual camera needs the DLL: the counter runs with the exe alone.

Windows cannot load a DirectShow filter from memory. `InprocServer32` stores a path and `CoCreateInstance` ends in `LoadLibrary` on that path, so a DLL that exists only inside the exe can never be the registered driver. It has to be written to disk before the first registration. The app would carry the DLL and unpack it only if the user wants the camera.

Sizes of the current build: `MeetBurnerCamera.dll` 142 KB, `meetburner.exe` 9.4 MB. Embedding costs 1.5 %.

The catch is reputational. An unsigned exe that writes a DLL to disk and then registers a COM server has exactly the shape of a dropper, and SmartScreen/AV heuristics are entitled to dislike it.

## Decision

### One binary, two layouts

The same `meetburner.exe` is published twice:

| Asset | Content |
|---|---|
| `MeetBurner_<version>_x64.exe` | the app, driver inside |
| `MeetBurner_<version>_portable_x64.zip` | the same exe + `resources\MeetBurnerCamera.dll` |

- No second build and no second signing pass: the zip carries a byte-identical exe.
- In the zip, `camera::driver::find` finds the file next to the exe first, so nothing is extracted. The single file extracts only because no file is there.
- Users choose: one file when their antivirus is relaxed, the zip when it is not.

### Embedding

- `build.rs` reads `src-tauri/resources/MeetBurnerCamera.dll` and, when it is a real PE image (`MZ`, larger than 1 KB), generates `EMBEDDED_DRIVER` with `include_bytes!` plus `EMBEDDED_DRIVER_VERSION`, parsed from `#define MB_CAMERA_DRIVER_VERSION <n>u` in `native/vcam-windows/common/meetburner_camera.h`.
- The committed placeholder is never embedded, so `cargo run`, `tauri dev` and the CI lint builds keep using the CMake output through `driver::dev_driver_path()`.
- The embedded bytes are the **signed** DLL, because the release job signs the file before `tauri build` (ADR-0025).
- Non-Windows builds embed nothing.

### Extraction

- Only when a file is needed: the first *Register webcam for meets*, or start-up for an installation that is already registered and older than the embedded driver (ADR-0020). Nothing is written while the camera is not registered.
- Target: `<exe dir>\MeetBurnerCamera.v<N>.dll`. The version in the name means a new build never has to overwrite a DLL a meeting app still has loaded.
- Written to `.tmp` and renamed, so a half-written driver is never registered. An existing file of the same length is reused as is.
- No `%LOCALAPPDATA%` fallback: a driver the user cannot see would make *clean* and support harder. A read-only folder (write-protected stick, `Program Files` without elevation) produces one explicit message in the Camera section.
- `DriverStatus` stays read-only and reports the path the first registration will use.

### Release notes say it

`release.yml` appends a *Portable builds* section to every release: what each file is, and that the zip is the recommended choice when the single file is flagged. The existing "not code signed" line is still added when no certificate was configured.

### Not chosen

- **The exe as its own `InprocServer32`**: an EXE is not a COM in-process server, and loading it as a module is unsupported.
- **An out-of-process filter (`LocalServer32`)**: DirectShow filters are expected in-process; a proxy filter is far more code than this problem deserves.
- **Extracting into `%LOCALAPPDATA%`**: always writable, but it breaks "delete the folder" and hides the file the registry points at.

## Consequences

- A release is one binary in two shapes; the installer is untouched and keeps `bundle.resources`.
- The portable promise holds: everything the app writes stays next to the exe, so *Clean webcam* plus deleting the file removes it.
- Updating the single file can leave `MeetBurnerCamera.v<old>.dll` behind while a meeting app holds it. The new build writes `v<N+1>`, registers it (ADR-0020) and the old file can be deleted once unlocked.
- The single file can still be quarantined until the certificate story is complete, which is why the zip ships next to it instead of being replaced.
