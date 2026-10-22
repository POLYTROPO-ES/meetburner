# 0016. DirectShow filter: hand-rolled COM and per-user registration

- Status: accepted
- Date: 2026-10-22

## Context

[ADR-0002](0002-virtual-camera.md) planned a DirectShow source filter registered by the installer with `regsvr32` (machine-wide, admin), inspired by the MIT `softcam` project. Two findings from implementing it:

1. The DirectShow **base classes** (`streams.h`, `CSource`/`CSourceStream`) are not part of a normal Windows SDK installation, so they cannot be linked without vendoring third-party sources.
2. Machine-wide registration is not needed: every meeting app runs as the signed-in user, and `HKCU\Software\Classes` is merged into `HKCR` for that user. Per-user registration needs no UAC and can be tested and removed at any time.

## Decision

### Filter implementation

- Hand-rolled COM, no base classes: `CameraFilter` (`IBaseFilter`, `IMediaFilter`, `IPersist`) and `CameraPin` (`IPin`, `IKsPropertySet`, `IAMStreamConfig`) with manual reference counting.
- The pin reports `PIN_CATEGORY_CAPTURE` through `IKsPropertySet`, which is how consumers choose a video capture pin.
- `IAMStreamConfig` is implemented (`GetFormat`, `SetFormat`, `GetNumberOfCapabilities`, `GetStreamCaps`) because consumers such as ffmpeg query the format there before selecting the pin.
- If the downstream pin has no allocator (`VFW_E_NO_ALLOCATOR`, e.g. ffmpeg's grabber), the source creates a `CLSID_MemoryAllocator` and calls `NotifyAllocator`.
- The pin advertises the producer's current size (`1280x720` before the app starts). If the producer changes size while a graph is running, frames are scaled (nearest neighbour) to the negotiated size so the stream never breaks.
- When nothing is published, the filter emits a GDI-rendered "MeetBurner / Camera not running" frame, as ADR-0002 requires.

### Registration and uninstall

- `DllRegisterServer` writes only under `HKCU\Software\Classes`:
  - `CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}` (name, `InprocServer32`, `ThreadingModel=Both`)
  - `...\Instance` (friendly name, CLSID)
  - `...\CLSID\{860BB310-5D01-11D0-BD3B-00A0C911CE86}\Instance\{...}` (the video input device category that meeting apps enumerate)
- `DllUnregisterServer` deletes both trees (`RegDeleteTreeW`), so `regsvr32 /u` leaves no trace.
- `scripts/register.ps1` and `scripts/unregister.ps1` wrap `regsvr32` and verify the keys afterwards.
- The DLL links the static CRT (`/MT`) because it is loaded inside other applications.
- This supersedes the **registration mechanism** of ADR-0002 (elevated machine-wide registration). The transport (ADR-0013) and the pipeline are unchanged.

## Consequences

- No admin prompt to install, try or remove the camera; the installer (M9) can call the same exports per user.
- `FilterData` blobs are not written: they embed in-process pointers and consumers such as ffmpeg, OBS and meeting apps enumerate the category instance by CLSID and `FriendlyName`.
- Property pages, `IAMStreamConfig2` and quality control are not implemented; add them only if a real consumer requires them.
- Verified locally with ffmpeg (`-f dshow -i video="MeetBurner Camera"`) at 1280x720 and 720x1280, 30 fps. The Zoom/OBS/Teams matrix remains a manual checklist item.
