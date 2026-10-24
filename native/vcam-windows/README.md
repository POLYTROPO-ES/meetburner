# vcam-windows

Owner: `vcam-windows` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md), [ADR-0013](../../docs/decisions/0013-windows-frame-ipc.md), [ADR-0016](../../docs/decisions/0016-dshow-filter-and-per-user-registration.md), [ADR-0019](../../docs/decisions/0019-camera-frame-orientation-and-mapping.md) and [ADR-0031](../../docs/decisions/0031-windows-mf-virtual-camera.md).

```
CMakeLists.txt
common/meetburner_camera.h   frame contract + seqlock reader (DLL and tests)
dshow/                       DirectShow source filter ("MeetBurner Camera")
dshow/frames.h               media type, bottom-up blit, "not running" frame
dshow/vcam.cpp               bottom-up RGB32 source filter
mf/                          Media Foundation virtual camera ("MeetBurner Camera (Windows 11)")
mf/mf_camera_api.h           MB_Mf* exports + CLSID + friendly name (mirrored in camera/mf.rs)
mf/mfvirtualcamera_api.h     minimal Windows 11 API declaration (runtime loaded)
mf/dll.cpp                  COM media source, per-user registration, MB_Mf* exports
mf/frames.cpp                top-down copy + aspect fit + placeholder
scripts/                     register.ps1 / unregister.ps1 (HKCU, no UAC)
tests/                       reader_test + frame_test + mf_frames_test (CTest), dshow_probe, graph_test
```

## What already exists (Rust producer)

`apps/desktop/src-tauri/src/camera/` produces the frames:

- `renderer.rs` draws cost + elapsed time into a 1280×720 BGRA frame.
- `ipc.rs` owns the frame protocol (header + seqlock).
- `sink/windows.rs` creates the mapping/event and publishes frames at 30 fps.

The DLLs only have to read. The contract below is frozen by ADR-0013.

## IPC contract

| Object | Name |
|--------|------|
| File mapping | `Local\MeetBurnerCamera` |
| Frame-ready event (auto-reset) | `Local\MeetBurnerCameraFrame` |

Mapping size: always `48 + 1920 * 1080 * 4` bytes, whatever the resolution (ADR-0019). Frames are BGRA, top-down in the mapping, no row padding; the header carries the size actually published.

The DLL exports `MeetBurnerCameraVersion()` and `DllRegisterServer` stores it as
`HKCU\…\CLSID\{DBD3A890-…}\MeetBurnerDriverVersion`; the app re-registers an older installation on
start (ADR-0020). Bump `MB_CAMERA_DRIVER_VERSION` in `common/meetburner_camera.h` whenever the filter
behaves differently.

```c
#pragma pack(push, 1)
typedef struct {
    uint32_t magic;          /* 0: 0x4D42434D ("MBCM") */
    uint32_t version;        /* 4: 1 */
    uint32_t width;          /* 8 */
    uint32_t height;         /* 12 */
    uint32_t fps;            /* 16 */
    uint32_t state;          /* 20: 0 = idle, 1 = live */
    uint64_t frame_counter;  /* 24 */
    uint64_t timestamp_ms;   /* 32 */
    uint32_t sequence;       /* 40: seqlock, odd while writing */
    uint32_t reserved;       /* 44: must be 0 */
    /* 48: BGRA payload */
} meetburner_frame_header;
#pragma pack(pop)
_Static_assert(sizeof(meetburner_frame_header) == 48, "header must stay 48 bytes");
```

### Reader algorithm (required)

1. `OpenFileMappingW(FILE_MAP_READ, FALSE, L"Local\\MeetBurnerCamera")`; if it fails, copy the static "MeetBurner – not running" frame.
2. Read the header; require `magic == 0x4D42434D` and `version == 1`.
3. If `sequence` is odd, retry (do not touch the payload).
4. Copy the payload, read the header again; accept only when the two `sequence` values are equal and even.
5. If `state == 0`, or `timestamp_ms` is more than 2 s behind the wall clock, show the "not running" frame; repeat the last good frame for at most ~2 s first, so a hiccup never flashes.

Readers must never write to the mapping and must never block: a stuck consumer cannot stall the producer.

### Resolution changes

The producer can change the output size while the app runs (16:9, 9:16, 4:3, 3:4 and 1:1 sizes, ADR-0015).
Since ADR-0019 the mapping is **never** re-created: only `width`/`height` in the header change, so a reader
keeps its view for the whole session. Re-read the header on every frame and scale the result into the
negotiated media type, keeping the aspect ratio (as `BlitFit` does).

### Row order

The two cameras disagree, so read this before touching either frame helper:

- **DirectShow** (`BlitFit`, `dshow/frames.h`): an RGB32 DIB with `biHeight > 0` is
  **bottom-up**, and it ignores a negative height, so the producer's top-down rows are flipped
  while copying; `RenderNotRunning` flips its GDI DIB too. `tests/frame_test.cpp` pins it.
  This is the ADR-0019 rule.
- **Media Foundation** (`FitBgra`, `mf/frames.h`): `MF_MT_DEFAULT_STRIDE` is positive, which
  means **top-down**, and a negative stride is not honoured, so the rows are copied in order
  and nothing is flipped. Flipping here rotated the picture 180° in every consumer
  (ADR-0031 amendment). `tests/mf_frames_test.cpp` pins the mapping.

In both cases the producer writes top-down BGRA (ADR-0013).

## Which apps see the device

The filter registers itself under `CLSID_VideoInputDeviceCategory` (`{860BB310-5D01-11D0-BD3B-00A0C911CE86}`),
the DirectShow camera list of the current Windows account. Media Foundation enumerates that list too
(through the frame-server bridge), so the device is not Chrome-only:

| Consumer | Capture stack | Camera to pick |
|----------|---------------|----------------|
| Chrome, Edge (Meet, Teams web, Discord web…) | Media Foundation, through the DirectShow bridge | **MeetBurner Camera** — same Chromium stack, same result |
| Firefox | DirectShow / Media Foundation backend | **MeetBurner Camera** |
| Zoom, Teams classic, Slack, Discord | DirectShow / Media Foundation bridge | **MeetBurner Camera** |
| OBS Studio | DirectShow | **MeetBurner Camera** |
| New Teams, Windows Camera app | Media Foundation only | **MeetBurner Camera (Windows 11)** (ADR-0031) |
| ffmpeg | DirectShow | **MeetBurner Camera** (verified) |

Verified so far: ffmpeg (DirectShow) and Google Meet in Chrome. The rest of the list is the M6 manual
matrix: same registration path, so a failure there is a bug in the filter, not in the visibility rules.

## Two cameras: DirectShow and Media Foundation

Windows 11 has a second camera that reads **the same shared memory**. Nothing in the producer
changes: both DLLs open `Local\MeetBurnerCamera` and either (or both) can be active. They have
different names so the camera pickers never show two identical entries:

| | DirectShow (`MeetBurnerCamera.dll`) | Media Foundation (`MeetBurnerMfCamera.dll`) |
|---|---|---|
| Friendly name | **MeetBurner Camera** | **MeetBurner Camera (Windows 11)** |
| Where it comes from | ADR-0016 source filter, registered per user (HKCU) | ADR-0031 COM media source + `MFCreateVirtualCamera` |
| Windows version | Windows 10 and 11 | Windows 11 build 22000+ only; reports unsupported elsewhere |
| Who needs it | Zoom, OBS, Chrome/Edge (Meet), Teams classic, ffmpeg | New Teams, Windows Camera app, other MF-only apps |
| Registration | App button / `regsvr32` / `scripts/register.ps1` | App commands (`camera_mf_register`) / `regsvr32` |
| Clean up | App button / `scripts/unregister.ps1` | App commands (`camera_mf_unregister`) / `regsvr32 /u` |

Chrome, Edge and Firefox use Media Foundation too; they find the **DirectShow** camera through
the frame-server bridge, which is why the DirectShow filter is still the one to try first. New
Teams and the Windows Camera app only enumerate MF cameras, so they need the second one.

### Build

The second target is part of the same CMake project and lands in the same folder and solution:

```powershell
cmake -S native/vcam-windows -B native/vcam-windows/build -A x64
cmake --build native/vcam-windows/build --config Release
# Release\MeetBurnerCamera.dll and Release\MeetBurnerMfCamera.dll
```

`mf/mfvirtualcamera_api.h` mirrors the few Windows 11 declarations the DLL uses, so the build
works with older Windows SDKs. `MFCreateVirtualCamera` is resolved at run time and never linked:
the DLL loads on Windows 10, where `MB_MfSupported()` returns 0.

### Register and remove

**Normal users do it from the app** (the `camera_mf_register` / `camera_mf_unregister` commands
behind the camera section). Both write/delete `HKCU\Software\Classes\CLSID\{9D6A2B41-…}` and
need no administrator prompt (ADR-0016/0017 approach). The command also creates the virtual
camera for the current Windows account with `MFVirtualCameraLifetime_System` and
`MFVirtualCameraAccess_CurrentUser`, then calls `Start()`; unregister calls `Remove()`.

Manual equivalent (for diagnostics): `regsvr32 /s <Release>\MeetBurnerMfCamera.dll` registers
the COM server only; creating the camera itself is what `MB_MfRegister` does, so use the app or
a tiny caller of the exports. The app status (`camera_mf_status`) calls `MB_MfSupported()` and
shows `supported: false` on Windows 10, where there is nothing to register.

### Verify the camera

1. Start the app (the counter is live while it is open, ADR-0018) and register the MF camera.
2. Open **Camera** (Windows Camera app): *MeetBurner Camera (Windows 11)* must appear in the
   camera picker and show the counter upright, not rotated.
3. New Teams: *Settings → Devices → Camera* must list it; the preview shows the counter.
4. The DirectShow camera must still work at the same time: open Meet in Chrome (or ffmpeg) and
   select **MeetBurner Camera**.

If the app lists the camera but a consumer cannot activate it, see the HKCU caveat in
[ADR-0031](../../docs/decisions/0031-windows-mf-virtual-camera.md): the frame server runs as
`Local Service`, and an HKLM registration would be the fallback (administrator required).

### What the MF media source does

- `IMFMediaSourceEx` (with `IMFGetService` and `IKsControl`) + `IMFMediaStream2`, each with its
  own `IMFMediaEventQueue`; one `PINNAME_VIDEO_CAPTURE` stream marked frame-server shareable.
- Media type `MFVideoFormat_RGB32`, 1280×720, 30 fps, positive stride (= top-down). The
  producer's top-down rows are copied in order, never flipped; a resolution change is
  letterboxed into the negotiated size (ADR-0019, ADR-0031 amendment).
- Pull model: `RequestSample` reads the newest frame (or the "not running" placeholder) and
  queues `MEMediaSample`. Sample times are QPC based and anchored on the header timestamp.
- No frame for 2 s or `state = 0` → the placeholder, never a blocked or torn preview.

## Build, register and test

```powershell
# x64 Release (MSVC); CMake from the VS 2022 install is fine
cmake -S native/vcam-windows -B native/vcam-windows/build -A x64
cmake --build native/vcam-windows/build --config Release
ctest --test-dir native/vcam-windows/build -C Release --output-on-failure
```

Register for the current user (no UAC) and remove every key again. **Normal users do this from the app** with the *Register webcam for meets* / *Clean webcam for meets* buttons (ADR-0017); the scripts remain for CI, diagnostics and manual testing:

```powershell
powershell -ExecutionPolicy Bypass -File native/vcam-windows/scripts/register.ps1
powershell -ExecutionPolicy Bypass -File native/vcam-windows/scripts/unregister.ps1
```

Test without opening the app: `produce_frames` publishes real rendered frames to the mapping.

```powershell
# terminal 1: 120 s of 1280x720 frames (or 720 1280 for portrait)
cd apps/desktop/src-tauri
cargo run --example produce_frames -- 120 1280 720

# terminal 2: the device shows up and delivers frames
ffmpeg -list_devices true -f dshow -i dummy
ffmpeg -f dshow -i video="MeetBurner Camera" -frames:v 1 -update 1 -y frame.png
```

Diagnostics, if a consumer does not list or connect the camera:

- `dshow_probe.exe` prints the pin direction, its capture category and the media types.
- `graph_test.exe` builds a graph with the Null Renderer and prints every HRESULT.

## Registration details

- Per-user (`HKCU\Software\Classes`), so meeting apps running as the same user see the device and no admin prompt is needed.
- Keys: our CLSID (+ `InprocServer32`, `Instance`) and an instance under the video input device category `{860BB310-5D01-11D0-BD3B-00A0C911CE86}`.
- `regsvr32 /u` (`scripts/unregister.ps1`) deletes both trees; run it before moving or removing the DLL.
- The DLL uses the static CRT, so consumers do not need the Visual C++ redistributable.

## Next steps

1. The app registers and cleans the DirectShow driver on demand (ADR-0017); it ships as a bundle resource and, in the one-file portable, embedded in the exe (ADR-0026). The installer does not register it.
2. Manual test matrix: Zoom, OBS, Chrome/Edge (Meet), Teams classic, new Teams, Windows Camera.
3. Verify the MF camera in new Teams and the Windows Camera app on a Windows 11 machine (M8); the code and its unit tests are done.

