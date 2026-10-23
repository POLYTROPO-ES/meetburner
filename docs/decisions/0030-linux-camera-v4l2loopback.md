# 0030. Linux camera: use v4l2loopback, never load it

- Status: accepted
- Date: 2026-10-24

## Context

ADR-0002 planned the Linux sink as "detect `v4l2loopback`, one-click setup via
`pkexec modprobe`" and named the `v4l` crate. Both assumptions turned out to be wrong for this
project:

- Loading the module from the app needs root, and a `modprobe` with different parameters
  **replaces** the module: every other application using a loopback device loses it. A meeting
  counter has no business restarting other people's cameras.
- `exclusive_caps=1` is what makes Chrome and Meet list a loopback device as a *camera* instead
  of a capture device; without it the feature looks broken even though frames are flowing.
- `v4l` is a large wrapper for what is six ioctls; `libc` is already in the tree.

## Decision

- The app **detects** an existing `v4l2loopback` device and uses it. It never loads or reloads
  the module; when none is present the setup page shows the command to run.
- Documented setup, required once per boot or via `/etc/modules-load.d/`:

  ```
  sudo modprobe v4l2loopback exclusive_caps=1 card_label="MeetBurner Camera"
  ```

- Detection reads `/sys/class/video4linux/video*/name` and prefers, in order: a card label
  containing `MeetBurner`, v4l2loopback's default `Dummy video device`, then the
  `MEETBURNER_V4L2_DEVICE` override. Capture-only devices (real webcams) are never selected.
- Frames are written as **YUYV** (`V4L2_PIX_FMT_YUYV`, BT.601 limited range) with one
  `VIDIOC_S_FMT` per size change and one `write()` per frame, through `libc` ioctls.
- The state reaches the UI through the `camera_v4l2_status` command
  (`{ available, device }`), false on every other platform.

## Consequences

- Linux users need one command before the first meeting, and again after a reboot unless they
  persist the module. `exclusive_caps=1` is part of the documented command, not an option.
- Secure Boot will refuse unsigned third-party modules on many distributions; the guide says so.
- Nothing in the app requires root, and the app cannot break other loopback users.
- Verification in Chrome/Meet, Zoom, OBS and Firefox is still pending on real hardware; the
  roadmap item stays open until the owner confirms it.
