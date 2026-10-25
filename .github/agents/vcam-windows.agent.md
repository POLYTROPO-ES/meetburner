---
description: 'MeetBurner Windows virtual camera: C++ DirectShow source filter (stage 1) and Media Foundation virtual camera (Windows 11) in native/vcam-windows, plus the Rust shared-memory producer.'
---

You work in `native/vcam-windows`, the Windows sink in `apps/desktop/src-tauri/src/camera/sink/windows.rs` and `camera/ipc.rs`, and the MF calls in `camera/mf.rs`.

## Scope
1. `dshow/`: DirectShow source filter DLL **MeetBurner Camera** (`MeetBurnerCamera.dll`), hand-rolled COM without the base classes, static CRT (ADR-0016).
2. `mf/`: in-proc Media Foundation media source **MeetBurner Camera (Windows 11)** (`MeetBurnerMfCamera.dll`, own CLSID) registered with `MFCreateVirtualCamera`, resolved at run time so both DLLs and the app still load on Windows 10 (ADR-0031).
3. IPC (ADR-0013): one mapping `Local\MeetBurnerCamera` with the 48-byte header (magic, version, width, height, fps, state, frame counter, timestamp, seqlock) and the top-down BGRA payload, plus the `Local\MeetBurnerCameraFrame` event. The mapping is created once at the largest size, so a resolution change only rewrites the header (ADR-0019); a mapping left by a dead producer is adopted (ADR-0024). `common/meetburner_camera.h` is the single C definition, mirrored in Rust with `#[repr(C)]`.
4. Rust `WinSharedMemSink` implementing `VirtualCameraSink`.
5. Registration (ADR-0016, ADR-0017, ADR-0020): per user under `HKCU\Software\Classes`, no admin; the app calls `DllRegisterServer` (or `MB_MfRegister`) from two buttons and cleans the keys itself. `MB_CAMERA_DRIVER_VERSION` in `common/meetburner_camera.h` is bumped whenever the filter behaves differently, so the app re-registers outdated installs on start. `scripts/*.ps1` are for development and diagnostics only.

## Rules
- TDD (ADR-0008): IPC header layout and the seqlock reader/writer are covered by Rust and CTest unit tests written before the code (`tests/`).
- Build with CMake + MSVC, x64; CI runs `cmake` + `ctest` (`.github/workflows/ci.yml`).
- Row order differs per camera: DirectShow RGB32 is bottom-up (`biHeight > 0`), so the filter flips; MF declares a positive stride (top-down) and must not flip (ADR-0019, ADR-0031 amendment). Never copy the flip from one camera to the other.
- When no producer is alive (mapping absent, `state = 0` or stale frames), output the static "not running" frame; never block the consumer app.
- The one-file portable exe embeds only the DirectShow DLL (ADR-0026); the installers and the zip carry both (`tools/copy-driver.mjs`).
- Pending (M6/M8): test matrix Zoom, OBS, Chrome/Edge (Meet), Teams classic (DirectShow); new Teams and Windows Camera (MF); embedding the MF DLL in the one-file portable.
