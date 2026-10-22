---
description: 'MeetCost Windows virtual camera: C++ DirectShow source filter (stage 1) and Media Foundation virtual camera (Windows 11) in native/vcam-windows, plus the Rust shared-memory producer.'
---

You work in `native/vcam-windows` and the Windows sink in `apps/desktop/src-tauri/src/camera/sink/windows.rs`.

## Scope
1. `dshow/`: DirectShow source filter DLL ("MeetCost Camera"), registered with `regsvr32`. Design inspired by the MIT *softcam* project (credit it if code is reused).
2. `mf/`: Media Foundation custom media source COM DLL + `MFCreateVirtualCamera` usage (Windows 11 22000+ only).
3. IPC: named shared memory `Local\MeetCostCamera` (header: magic, version, width, height, fps, frame counter, timestamp; then BGRA frame) + named event for new frames.
4. Rust `WinSharedMemSink` implementing `VirtualCameraSink`.

## Rules
- TDD (ADR-0008): IPC header layout and ring-buffer logic are covered by Rust and CTest unit tests written before the code.
- Build with CMake + MSVC, x64. Keep the IPC header in one shared file used by both DLLs; mirror it in Rust with `#[repr(C)]`.
- When no producer is alive, output a static "not running" frame; never block the consumer app.
- Installer must register and unregister cleanly; document admin requirement.
- Test matrix: Zoom, OBS, Chrome/Edge (Meet), Teams classic (DirectShow); new Teams and Windows Camera (MF).
