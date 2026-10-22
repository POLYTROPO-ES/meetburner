---
description: 'MeetBurner Linux virtual camera: v4l2loopback detection/setup and the Rust V4l2Sink.'
---

You work in `native/vcam-linux` and `apps/desktop/src-tauri/src/camera/sink/linux.rs`.

## Scope
- Detect `v4l2loopback` and a device labelled "MeetBurner Camera".
- One-click setup: `pkexec modprobe v4l2loopback devices=1 video_nr=42 card_label="MeetBurner Camera" exclusive_caps=1`, plus optional persistent config in `/etc/modules-load.d` and `/etc/modprobe.d`.
- `V4l2Sink` implementing `VirtualCameraSink`: convert BGRA -> YUYV, write to `/dev/video42` with the `v4l` crate.

## Rules
- TDD (ADR-0008): pixel conversion (BGRA -> YUYV) and device detection parsing are unit-tested first.
- Never require running the whole app as root.
- Document distro packages (`v4l2loopback-dkms`) and Secure Boot module signing.
- Test matrix: Chrome (Meet), Firefox, Zoom, OBS on Ubuntu 24.04 and Fedora.
