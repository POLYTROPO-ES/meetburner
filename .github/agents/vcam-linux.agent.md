---
description: 'MeetBurner Linux virtual camera: v4l2loopback detection (never loading the module) and the Rust V4l2Sink.'
---

You work in `native/vcam-linux` (setup docs) and `apps/desktop/src-tauri/src/camera/sink/linux.rs`. Read ADR-0030 first: it replaces the Linux setup planned in ADR-0002.

## Scope
- Detect an existing `v4l2loopback` device through `/sys/class/video4linux/video*/name`, in order: a card label containing `MeetBurner`, v4l2loopback's default `Dummy video device`, then the `MEETBURNER_V4L2_DEVICE` override. Never select a capture-only device (a real webcam).
- `V4l2Sink` implementing `VirtualCameraSink`: convert BGRA -> YUYV (BT.601 limited range), one `VIDIOC_S_FMT` per size change, one `write()` per frame, through raw `libc` ioctls (no `v4l` crate).
- `camera_v4l2_status` returns `{ available, device }`; without a device the engine uses `NullSink` and the app keeps running.
- Setup documentation in `native/vcam-linux/README.md` and the user guide: the `modprobe … exclusive_caps=1 card_label="MeetBurner Camera"` command, persistence in `/etc/modules-load.d` + `/etc/modprobe.d`, distro packages (`v4l2loopback-dkms`) and Secure Boot module signing.

## Rules
- TDD (ADR-0008): pixel conversion (BGRA -> YUYV) and device detection parsing are unit-tested first.
- The app never loads, reloads or reconfigures the kernel module (no `pkexec`/`modprobe` from the app): it needs root and would take the device away from other loopback users. Show the command instead.
- `exclusive_caps=1` is part of the documented command, not an option: without it Chrome/Meet do not list the device.
- Pending (M5): test matrix Chrome (Meet), Firefox, Zoom, OBS on Ubuntu 24.04 and Fedora; only the owner can tick it after real-hardware checks.
