# vcam-windows

Owner: `vcam-windows` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md) and [ADR-0013](../../docs/decisions/0013-windows-frame-ipc.md).

Planned layout:

```
common/   shared-memory IPC header (C), mirrored in Rust with #[repr(C)]
dshow/    DirectShow source filter DLL – stage 1 (M6)
mf/       Media Foundation virtual camera – Windows 11 (M8)
CMakeLists.txt
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

Mapping size: `48 + width * height * 4` bytes. Frames are 1280×720 BGRA, top-down, no row padding.

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
5. If `state == 0` or no frame arrived in ~1 s, output the "not running" frame.

Readers must never write to the mapping and must never block: a stuck consumer cannot stall the producer.

## Next steps

1. `common/meetburner_camera.h` with the struct above + the reader loop.
2. `dshow/` source filter registering as "MeetBurner Camera" (x64, `regsvr32`), plus registration/unregistration in the NSIS/MSI installer.
3. `mf/` media source for Windows 11 (`MFCreateVirtualCamera`).
4. Manual test matrix: Zoom, OBS, Chrome/Edge (Meet), Teams classic, new Teams, Windows Camera.

