# 0002. Virtual camera per OS

- Status: accepted
- Date: 2026-10-22

## Context

The key feature: when the user hits **Start**, a system camera named "MeetCost Camera" shows the counter, either alone or overlaid on a real webcam. Meeting apps (Teams, Zoom, Meet in Chrome/Edge, Slack…) must be able to select it.

## Decision

### Common pipeline (Rust, in the Tauri app)

```
[nokhwa source cam]? -> compositor (tiny-skia) -> Frame (1280x720 @30fps, BGRA/NV12/YUYV)
                                                 -> dyn VirtualCameraSink
```

```rust
trait VirtualCameraSink {
    fn start(&mut self, width: u32, height: u32, fps: u32) -> Result<()>;
    fn send(&mut self, frame: &Frame) -> Result<()>;
    fn stop(&mut self);
}
```

### Windows (stage 1)

1. **DirectShow source filter** (`native/vcam-windows/dshow`, C++ DLL, design inspired by the MIT-licensed *softcam* project).
   - Registered once with `regsvr32` by the installer (admin).
   - Frames passed through a named shared-memory ring buffer (`Local\MeetCostCamera`) + named event.
   - Visible to DirectShow consumers (Zoom, OBS, many Electron apps, Chrome fallback).
2. **Media Foundation virtual camera** (`native/vcam-windows/mf`, Windows 11 22000+).
   - COM media source DLL + `MFCreateVirtualCamera` from the app.
   - Reads the same shared memory. Required for apps that only enumerate MF devices (new Teams, Windows Camera app).
   - Disabled automatically on Windows 10.

When no producer is running, both sinks emit a static "MeetCost – not running" frame.

### Linux (stage 1)

- `v4l2loopback` kernel module: `modprobe v4l2loopback devices=1 video_nr=42 card_label="MeetCost Camera" exclusive_caps=1`.
- App offers a one-click setup via `pkexec` and documents the distro package (`v4l2loopback-dkms`).
- Rust writes YUYV frames to `/dev/video42` with the `v4l` crate.

### macOS (stage 2)

- CoreMediaIO **Camera Extension** (macOS 12.3+) in Swift, embedded in the app bundle, activated with `OSSystemExtensionRequest`.
- App pushes frames via the extension's sink stream.
- Requires Apple Developer ID, system-extension entitlement, notarization.

### Overlay

- User selects source webcam, overlay position (corner/top/bottom bar), scale and opacity.
- The physical webcam is opened by MeetCost; meeting apps select "MeetCost Camera" instead.

## Consequences

- Windows installer needs admin once (DLL registration). Code signing strongly recommended (SmartScreen, AV false positives).
- Linux needs a kernel module; Secure Boot systems need DKMS signing (document it).
- Two Windows implementations to maintain, sharing the IPC layer.
