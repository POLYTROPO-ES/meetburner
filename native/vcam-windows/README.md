# vcam-windows

Owner: `vcam-windows` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md), [ADR-0013](../../docs/decisions/0013-windows-frame-ipc.md), [ADR-0016](../../docs/decisions/0016-dshow-filter-and-per-user-registration.md) and [ADR-0019](../../docs/decisions/0019-camera-frame-orientation-and-mapping.md).

```
CMakeLists.txt
common/meetburner_camera.h   frame contract + seqlock reader (DLL and tests)
dshow/                       DirectShow source filter ("MeetBurner Camera")
dshow/frames.h               media type, bottom-up blit, "not running" frame
dshow/vcam.cpp               bottom-up RGB32 source filter
scripts/                     register.ps1 / unregister.ps1 (HKCU, no UAC)
tests/                       reader_test + frame_test (CTest), dshow_probe, graph_test
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

RGB32 reaches consumers **bottom-up** (`biHeight > 0`): Media Foundation — Chrome, Meet, new Teams —
ignores a negative height and shows a top-down buffer upside down, which made the counter look rotated 180°.
The producer writes top-down, the filter flips while copying (`BlitFit`) and `RenderNotRunning` flips its
GDI DIB too. `tests/frame_test.cpp` pins both.

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

1. Bundle the DLL and run the register/unregister exports from the NSIS/MSI installer (M9).
2. Manual test matrix: Zoom, OBS, Chrome/Edge (Meet), Teams classic, new Teams, Windows Camera.
3. `mf/` media source for Windows 11 (`MFCreateVirtualCamera`, M8).

