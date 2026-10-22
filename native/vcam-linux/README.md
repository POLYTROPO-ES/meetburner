# vcam-linux

Owner: `vcam-linux` agent. See [ADR-0002](../../docs/decisions/0002-virtual-camera.md).

Manual setup (what the app automates in M5):

```bash
sudo apt install v4l2loopback-dkms        # Debian/Ubuntu
sudo dnf install v4l2loopback             # Fedora (RPM Fusion)
sudo modprobe v4l2loopback devices=1 video_nr=42 card_label="MeetCost Camera" exclusive_caps=1
```

The app writes frames to `/dev/video42`.
