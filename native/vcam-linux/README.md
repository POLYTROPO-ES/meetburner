# vcam-linux

Owner: `vcam-linux` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md).

MeetBurner publishes the counter to a [v4l2loopback](https://github.com/umlaeute/v4l2loopback)
device labelled **MeetBurner Camera**. Meeting apps see it as a normal webcam.

The app **detects an existing device and never loads kernel modules**: that step needs root, so
it stays with the user or the distro package. Without a device the app keeps running, discards
the camera frames and `camera_v4l2_status` returns `{ "available": false, "device": null }`.

## 1. Install the module

| Distro | Package |
|---|---|
| Ubuntu / Debian | `sudo apt install v4l2loopback-dkms` (add `linux-headers-$(uname -r)` if DKMS cannot find the headers) |
| Fedora (RPM Fusion) | `sudo dnf install v4l2loopback` (akmod) |
| Arch / Manjaro | `sudo pacman -S v4l2loopback-dkms` |

These packages build the module with DKMS, so it follows kernel updates.

## 2. Load it

```bash
sudo modprobe v4l2loopback devices=1 video_nr=42 card_label="MeetBurner Camera" exclusive_caps=1
```

- `devices=1` — one loopback camera.
- `video_nr=42` — fixed `/dev/video42`, so the same node comes back after a reboot.
- `card_label="MeetBurner Camera"` — the name meeting apps show in their camera picker.
- `exclusive_caps=1` — **required**. Without it v4l2loopback advertises the node with capture
  and output capabilities, and Chrome (therefore Google Meet) does not list it as a camera.

If `/dev/video42` is taken, use another number: the app looks for the card label, not the node
number (see *Detection*).

### Keep it across reboots

```bash
echo v4l2loopback | sudo tee /etc/modules-load.d/meetburner.conf
sudo tee /etc/modprobe.d/meetburner.conf <<'EOF'
options v4l2loopback devices=1 video_nr=42 card_label="MeetBurner Camera" exclusive_caps=1
EOF
```

Stop MeetBurner, then `sudo modprobe -r v4l2loopback` before reloading after a change.

### Secure Boot

The kernel only loads signed modules. Ubuntu's DKMS setup creates a Machine Owner Key:

```bash
sudo update-secureboot-policy --enroll-key   # choose a one-time password
# reboot, then enroll the key in the blue MOK screen
```

On Fedora:

```bash
sudo kmodgenca -a
sudo mokutil --import /etc/pki/akmods/certs/public_key.der
# reboot and enroll the key, then:
sudo akmods --force
```

If `modprobe` says `Key was rejected by service`, the module is unsigned: enroll the key or
disable Secure Boot (`dmesg | tail` shows the rejection).

## 3. Verify

```bash
ls -l /dev/video42
cat /sys/class/video4linux/video42/name   # MeetBurner Camera
v4l2-ctl --list-devices                   # optional, package v4l-utils
```

Start the camera in MeetBurner, then capture one frame:

```bash
ffmpeg -f v4l2 -i /dev/video42 -frames:v 1 -y out.png
```

`out.png` must show the counter. In Chrome open a Google Meet preview → *Settings → Video* and
pick **MeetBurner Camera**; the counter must already be visible before the meeting starts
(`00:00:00`, ADR-0018). Firefox lists the same device in its camera permission dropdown.

The M5 test matrix is Chrome (Meet), Firefox, Zoom and OBS on Ubuntu 24.04 and Fedora; it is
still pending verification on real machines (`docs/plan/roadmap.md`).

## Detection and overrides

The app reads `/sys/class/video4linux/video*/name` and picks, in order:

1. a card whose name contains `MeetBurner`;
2. a card whose name contains `Dummy video device` — v4l2loopback's default label when the
   module is loaded without `card_label`;
3. the devnode in `MEETBURNER_V4L2_DEVICE`, for a custom label:

   ```bash
   MEETBURNER_V4L2_DEVICE=/dev/video9 meetburner
   ```

Real webcams are ignored: they are capture-only, so writing frames to them cannot work.

## Implementation

`apps/desktop/src-tauri/src/camera/sink/linux.rs`:

- `V4l2Sink` implements `VirtualCameraSink`: `open(O_RDWR)`, `VIDIOC_S_FMT` with
  `V4L2_BUF_TYPE_VIDEO_OUTPUT` and `V4L2_PIX_FMT_YUYV` at the requested size, then one `write()`
  per frame; `stop()` closes the descriptor. The render loop writes at 30 fps.
- BGRA is converted to packed YUYV (BT.601 limited range, 2 bytes per pixel); a resolution
  change re-issues `VIDIOC_S_FMT` on the next frame instead of reopening the device.
- `loopback_devices(root)` parses the sysfs tree (tests use a temp directory) and `device()` is
  the thin `/sys/class` + `MEETBURNER_V4L2_DEVICE` wrapper.
- `engine.rs` keeps `NullSink` when no device is found, so the rest of the app works exactly as
  before M5.

No root is needed to run the app: only installing and loading the module does.
