# 0013. Windows camera frame IPC (shared memory + seqlock)

- Status: accepted
- Date: 2026-10-22

## Context

The Tauri app is the single producer of camera frames; the future DirectShow (M6) and Media Foundation (M8) DLLs are consumers living in other processes. They need a transport that is fast (30 fps of 1280×720 BGRA ≈ 110 MB/s), works without admin rights, and never makes a meeting app hang or show a torn frame.

[ADR-0002](0002-virtual-camera.md) only fixed the transport family (named shared memory + named event). This ADR fixes the exact layout so the native DLLs can be written against a frozen contract.

## Decision

### Names

| Object | Name |
|--------|------|
| File mapping | `Local\MeetBurnerCamera` |
| Frame-ready event (auto-reset, manual-reset = false) | `Local\MeetBurnerCameraFrame` |

The `Local\` prefix scopes both objects to the user session, so they need no privileges. The app is the only producer: if `CreateFileMappingW` reports `ERROR_ALREADY_EXISTS`, `camera_start` fails instead of silently sharing its frames with another instance.

### Layout

One mapping of `48 + width × height × 4` bytes: a fixed header followed by tightly packed, top-down, 32-bit BGRA pixels (no row padding).

```c
#pragma pack(push, 1)
typedef struct {
    uint32_t magic;          /*  0: 0x4D42434D ("MBCM") */
    uint32_t version;        /*  4: 1 */
    uint32_t width;          /*  8 */
    uint32_t height;         /* 12 */
    uint32_t fps;            /* 16 */
    uint32_t state;          /* 20: 0 = idle, 1 = live */
    uint64_t frame_counter;  /* 24: increments once per produced frame */
    uint64_t timestamp_ms;   /* 32: Unix epoch ms of the frame */
    uint32_t sequence;       /* 40: seqlock, odd while writing */
    uint32_t reserved;       /* 44: must stay 0 */
    /* 48: BGRA payload */
} meetburner_frame_header;
#pragma pack(pop)
```

`#[repr(C)]` in Rust and `#pragma pack(1)` in C give the same 48-byte header; both sides assert the size at compile time.

### Writing (producer)

1. Bump `sequence` to an odd value and publish the header (payload is now untrusted).
2. Copy the whole `width × height × 4` payload.
3. Publish an even `sequence`, the new `frame_counter` and `timestamp_ms`.
4. `SetEvent` the frame-ready event.

### Reading (consumer)

1. Read the header; bail out (and show the "not running" frame) if `magic`/`version` do not match.
2. Retry while `sequence` is odd.
3. Copy the payload, read the header again; accept only when the two `sequence` values match and are even.

Readers never write to the mapping and never block the producer, so a stuck or crashed consumer cannot stall rendering.

### Lifecycle

- `camera_stop` writes `state = 0` (keeping the last frame) and then closes the mapping; consumers that still hold a handle see `state = 0`.
- Consumers show a static "MeetBurner – not running" frame when the mapping is absent, `state = 0`, or no frame arrived for ~1 s.
- A crashed producer leaves the mapping to the OS (destroyed with the last handle); consumers fall back on the timeout rule.

## Consequences

- The header is versioned; adding fields requires `version = 2` and a coordinated DLL update.
- 3.5 MB of shared memory per producer; negligible.
- The DLLs must implement the reader loop above; the Rust side is covered by unit tests (`camera::ipc`) and a real round-trip test through the mapping.
- The contract is documented for the native implementation in [native/vcam-windows/README.md](../../native/vcam-windows/README.md).
