# 0031. Windows Media Foundation virtual camera

- Status: accepted
- Date: 2026-10-24

## Context

The DirectShow source filter (ADR-0016) is enough for Zoom, OBS, Chrome/Edge (Meet), Teams
classic and ffmpeg, but some Windows 11 apps only look at Media Foundation cameras: the new
Teams and the Windows Camera app are the two that matter. Windows 11 21H2 (build 22000)
added `MFCreateVirtualCamera`, which registers a user-mode software camera into the frame
server pipeline; it does not exist on Windows 10.

ADR-0002 already planned this second camera and required both to read the same shared
memory. The producer (`WinSharedMemSink`, ADR-0013/0019) must not change: it keeps writing
BGRA frames into `Local\MeetBurnerCamera`, so either camera — or both — can be active while
the app runs.

## Decision

### A second camera, not a replacement

- DirectShow stays exactly as it is: same DLL, same CLSID, same friendly name
  **MeetBurner Camera**.
- The new DLL is `native/vcam-windows/mf/MeetBurnerMfCamera.dll`, with its own CLSID
  (`{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}`) and the friendly name
  **MeetBurner Camera (Windows 11)**. Meeting apps therefore never list two entries with the
  same name; the user picks the one that matches the app they use.
- `camera_mf_status` reports whether the MF camera is `supported` (Windows 11 and the DLL is
  present) and `registered`, so the UI can explain "requires Windows 11" instead of hiding
  the feature.

### Same shared memory, no new transport

The DLL opens the existing mapping and uses the same reader algorithm as the DirectShow
filter (`common/meetburner_camera.h`). Consequences:

- The producer is untouched: starting the counter feeds both cameras.
- A crashed app leaves the section behind; the MF camera shows the "not running" frame after
  the same 2 s staleness window (ADR-0019).
- Resolution changes are free: the header carries the size and the sample is letterboxed
  into the negotiated media type.

### Media source shape

- In-proc COM server exporting `DllGetClassObject`/`DllCanUnloadNow` for the CLSID, because
  the frame server activates the source by CLSID.
- `IMFMediaSourceEx` (with the mandatory `IMFGetService` and `IKsControl`) and
  `IMFMediaStream2`, each with its own `IMFMediaEventQueue`, as the frame-server custom
  media source contract requires.
- One stream: `MF_DEVICESTREAM_STREAM_CATEGORY = PINNAME_VIDEO_CAPTURE`,
  `MF_DEVICESTREAM_STREAM_ID = 0`, `MF_DEVICESTREAM_FRAMESERVER_SHARED = 1` (both cameras may
  serve the same app) and `MF_DEVICESTREAM_ATTRIBUTE_FRAMESOURCE_TYPES = Color`.
- Media type `MFVideoFormat_RGB32`, 1280×720, 30/1, `MF_MT_DEFAULT_STRIDE = +5120`. RGB32 is
  bottom-up in Media Foundation and MF ignores a negative stride, so the producer's top-down
  rows are flipped while the sample is filled (ADR-0019). `mf/frames.{h,cpp}` owns the flip
  and the aspect-fit, and `tests/mf_frames_test.cpp` pins them.
- Pull model: every `RequestSample` reads the newest frame (or the placeholder), allocates
  the buffer with `MFCreateMemoryBuffer`, fills it and queues `MEMediaSample` with the
  token. No worker thread, no queue beyond the event queue.
- Sample times: QPC based, anchored on the frame header `timestamp_ms`
  (`MFGetSystemTime()` at the first frame plus the header delta), always strictly increasing;
  the placeholder advances by one frame duration. This keeps the frame header as the clock
  the meeting app sees and satisfies the frame server's QPC requirement.

### Registration and lifetime

- `MB_MfRegister` writes the CLSID + `InprocServer32` keys under
  `HKCU\Software\Classes`, so no administrator prompt is needed (same approach as
  ADR-0016/0017). `regsvr32` works too because `DllRegisterServer` is exported.
- It then calls `MFCreateVirtualCamera` with the source CLSID, lifetime
  `MFVirtualCameraLifetime_System` (the camera survives restarts) and access
  `MFVirtualCameraAccess_CurrentUser` (per Windows account), followed by
  `IMFVirtualCamera::Start`. `MB_MfUnregister` creates the same object, calls `Remove()` and
  deletes the HKCU tree, including the `MeetBurnerMfRegistered` marker the status reads.
- The app calls the three exports through `LoadLibraryW` + `GetProcAddress`
  (`apps/desktop/src-tauri/src/camera/mf.rs`), never through the import library, so the DLL
  loads on Windows 10.

### Dynamic loading and the Windows 11 gate

`MFCreateVirtualCamera` lives in `mfsensorgroup.dll` (documented) and is resolved with
`LoadLibraryW("mfvirtualcamera.dll")` first, then `LoadLibraryW("mfsensorgroup.dll")`. The
DLL is built for Windows 10 (`_WIN32_WINNT=0x0A00`) and never links the function, so:

- the app and the DLL keep working on Windows 10, where `MB_MfSupported()` returns 0 and
  registering returns `HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED)`;
- the build does not need the Windows 11 SDK header; `mf/mfvirtualcamera_api.h` mirrors the
  few declarations used (interface IID and vtable order must match the SDK);
- the Rust side also gates on the OS build (`RtlGetVersion`, build ≥ 22000), so the status
  is correct even before the DLL is loaded.

## Consequences

- Two registrations to maintain and two "clean" paths; the same button pair in the app will
  eventually handle both, and `tools/copy-driver.mjs` now ships both DLLs.
- The one-file portable build embeds only the DirectShow driver (ADR-0026): the MF camera
  needs the installer or the zip, which carry `resources\MeetBurnerMfCamera.dll`.
- Windows 10 users are unaffected: the MF camera reports unsupported and the DirectShow
  filter keeps serving every app.
- The camera is per Windows account, like the DirectShow registration. Each account registers
  once; `Remove` + re-register is the recovery path.
- HKCU registration is what the task asked for and matches ADR-0016/0017. The frame server
  runs as `Local Service` in session 0; if a future Windows build refuses an HKCU in-proc
  server there, the fallback is an HKLM registration, which needs administrator rights. This
  is the first thing to check if Windows Camera or new Teams list the camera but cannot
  activate it.
- Windows appends "Windows Virtual Camera" to the friendly name in some pickers; the
  MeetBurner prefix still tells the cameras apart.

## Amendment (2026-10-24)

The first implementation got the row order wrong. It declared
`MF_MT_DEFAULT_STRIDE = +5120` — a positive stride, which means **top-down** to Media
Foundation — and then flipped every row to bottom-up while filling the sample. Live frames
therefore reached consumers rotated 180°: the counter card that belongs bottom-right appeared
top-left with the text upside down. The "not running" placeholder, generated from a top-down
DIB and copied as-is, was upright; that difference is what made the bug obvious.

The payload is top-down and stays top-down: `mf/frames.cpp` copies row `n` of the source to
row `n` of the fitted band (only the aspect-fit bars and the centring move it), under the same
positive stride. `FitFlipBgra` is now `FitBgra` so the name stops implying a flip, and
`tests/mf_frames_test.cpp` asserts that the first source row lands in the first destination row
and the last in the last, including when letterbox bars are present.

ADR-0019's bottom-up rule applies to the **DirectShow filter only**: a DirectShow RGB32 DIB
with a positive `biHeight` is bottom-up, so `BlitFit` must flip. Media Foundation is the
opposite convention and must not. Do not copy the flip from one camera to the other.
