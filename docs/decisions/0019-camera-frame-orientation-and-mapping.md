# 0019. Camera frames: bottom-up rows, one mapping and no flicker

- Status: accepted
- Date: 2026-10-22

## Context

First real call using "MeetBurner Camera" in Google Meet surfaced three defects:

1. The counter appeared rotated 180° in the preview (text mirrored and the layout upside down).
2. Changing the output resolution lost the device: the meeting app had to switch the camera off and on again.
3. Now and then the preview flashed the "Camera not running" placeholder for a frame.

Causes:

1. ADR-0013 frames are top-down BGRA and the pin declared them with a negative `biHeight`. ffmpeg honours the sign (its capture was correct), but Media Foundation — which is what Chrome, Meet and new Teams use — treats RGB32 as bottom-up and ignores the sign, so the image arrived upside down. Meet also mirrors your own preview, so the 180° rotation looked like a plain mirror.
2. ADR-0015 re-created the shared-memory mapping on every resolution change (`write_idle` + unmap + `CreateFileMappingW`). Consumers keep the view they opened, so they kept reading the orphaned section (frozen image) or lost the device, and `ProducerSize` changed under their feet.
3. That same stop/start set `state = idle`, so the filter painted the placeholder for a frame or two.

## Decision

### Bottom-up frames

- `MakeVideoType` declares `biHeight = height` (positive, bottom-up).
- The producer keeps publishing top-down (the renderer's natural order); the filter flips while it copies (`BlitFit`), and `RenderNotRunning` flips its GDI DIB the same way.
- `native/vcam-windows/dshow/frames.h` is the shared contract, and `tests/frame_test.cpp` pins the row order so this cannot silently regress.

### One mapping per app run

- The mapping is always created with `CAPACITY_LEN` = header + 1920×1080×4 bytes (the largest supported size; 1080×1920 has the same pixel count).
- The frame header carries the real size, so a resolution change only changes the numbers in the header: no unmap, no `CreateFileMappingW`, no `state = idle`, and readers keep one valid view for the whole session.
- The filter scales the frame into the negotiated media type and keeps the aspect ratio (`BlitFit` letterboxes with the theme background). A meeting app therefore never loses the device, whatever the producer publishes.

### No flicker

- The placeholder is only painted when the producer reports `STATE_IDLE` (the app stopped or the driver was cleaned) or when the timestamp is more than 2 s old — which is also what happens after a crash, because the section outlives the producer while readers hold it.
- A transient read failure (seqlock retry exhausted, mapping not yet open) repeats the last delivered frame, for at most ~2 s.

### Help text

The setup page explains that meeting apps keep the size they negotiated when they opened the camera and that the counter is always scaled to fit, so reselecting the camera is only needed to change the frame shape.

## Consequences

- The preview is upright in Meet, Chrome, new Teams and ffmpeg alike.
- Resolution changes are live. The negotiated frame size stays until the app reselects the camera, which is a DirectShow/Media Foundation constraint, not a bug.
- The mapping is ~8 MiB (1920×1080×4) instead of exactly the current size; in exchange, resize is free and readers never touch a freed view.
- The letterbox bars use the theme background, so a portrait counter inside a landscape negotiation still looks intentional.
