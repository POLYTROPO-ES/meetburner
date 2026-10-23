# 0024. Adopt a camera mapping whose producer is gone

- Status: accepted
- Date: 2026-10-23

## Context

`Local\MeetBurnerCamera` is a named section created by the app. Windows keeps it alive while **any**
process holds a view of it, and meeting apps hold that view for as long as their capture graph runs.
So when the app is killed (crash, task manager, installer) while a meeting app still has "MeetBurner
Camera" selected, the section survives without a producer.

The next app start then got `ERROR_ALREADY_EXISTS` and reported *"another MeetBurner camera is already
running"* — forever, until the user restarted the meeting app. Two instances of the app are still
supposed to be mutually exclusive, so simply ignoring the error was not an option.

## Decision

- `CreateFileMappingW` returning `ERROR_ALREADY_EXISTS` no longer fails immediately. The sink opens the
  existing section (`OpenFileMappingW` + `MapViewOfFile`) and reads the frame header.
- The mapping is **adopted** only when it is provably abandoned:
  - the header is not ours (`magic`/`version` mismatch) → nothing was ever published, or
  - its `timestamp_ms` is more than 5 s old → the producer stopped writing (the producer writes at
    30 fps, so a live one is always fresh).
- Otherwise the start fails exactly as before: a live producer keeps the camera.
- Adopting is logged (`taking over the abandoned camera mapping`) and the sink then owns the section
  exactly like if it had created it: `write_idle`, frame event, and normal `stop`/`Drop` cleanup.

## Consequences

- A crashed app no longer poisons the camera: the next start recovers on its own, at most 5 s after
  the last frame of the previous producer.
- Two live instances are still refused, because the running one keeps the timestamp fresh.
- The 5 s window is a trade-off: a producer paused for longer than that (a suspended laptop, a
  debugger breakpoint) could be adopted by a second instance. Both would then write into the same
  section, and the visible symptom is a flickering counter — recoverable by stopping one of them.
- `ipc::peek_header` exposes the header read used by this check, and a unit test rewrites the header
  timestamp of a live section to prove the takeover.
