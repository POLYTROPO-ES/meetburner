# 0018. The virtual camera is live while the app is open

- Status: accepted
- Date: 2026-10-23

## Context

Until now the camera published frames only between *Start* and *Stop* (ADR-0015). Meeting apps build their device list and their preview when the user opens the camera settings, and several of them do not refresh a device that appears later. A device that only exists after pressing Start cannot be previewed, so the user cannot check that MeetBurner is sending the counter before joining the call.

## Decision

### Lifecycle

The camera pipeline (render thread + shared-memory producer) runs from the moment the driver is registered until the app closes or the user cleans the driver:

| State | Camera |
|-------|--------|
| Driver registered (one click, ADR-0017) | Live: "MeetBurner Camera" always has frames |
| Driver cleaned | Stopped: the mapping is closed and the device shows the "Camera not running" placeholder |
| App closed | Stopped: the process that owns the mapping exits |

`camera_start` is called as soon as `camera_driver_status` reports `registered`; `camera_stop` is called when the driver is cleaned or the setup page unmounts.

### Idle state before the meeting starts

`cameraInput` in `packages/core/src/camera.ts` no longer returns `null`. Without a `startAt` it sends a zero-length window (`startAt = endAt = now`), which the renderer shows as `00:00:00` and `0,00 €` and never advances. That is exactly the frame a meeting that just started shows, and it lets the meeting app preview the theme, the fonts and the resolution.

### Errors do not stop the app

`useCameraSync` returns the failure message instead of only logging it. The setup page shows it under the camera buttons (for example, when another MeetBurner instance already owns the mapping) while the rest of the app keeps working; the next parameter change retries.

### Removed

The *Send the counter to "MeetBurner Camera"* checkbox and the "Start the meeting to turn the camera on" hint are gone: the camera is not an option gated by the meeting any more, it is the reason to keep the app open.

## Consequences

- The user opens the meeting app, picks "MeetBurner Camera" and previews the counter before the meeting starts; no start/pause dance.
- The render thread runs while the app is open (one frame every 33 ms at 1280×720); the CPU cost is the same as during a meeting.
- Only one MeetBurner process can own the mapping (`ERROR_ALREADY_EXISTS`); the second one shows the error and keeps running without a camera.
- Stored camera settings keep only `width`/`height`; entries written with `enabled` are ignored by `normalizeCamera`.
