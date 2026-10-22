# 0020. The app updates an outdated driver on start

- Status: accepted
- Date: 2026-10-22

## Context

ADR-0019 changed the filter's behaviour (bottom-up frames, one mapping, stale-frame handling). A
registration made before that keeps pointing at the old DLL: without an update the user keeps the
broken camera until they press *Clean webcam for meets* and register again. Asking a meeting attendee
to do that after every release is not acceptable, and nothing in the UI said the installed driver was
old.

## Decision

### Version

- `MB_CAMERA_DRIVER_VERSION` lives in `native/vcam-windows/common/meetburner_camera.h` and is bumped
  whenever the filter behaves differently (`1` = first release, `2` = ADR-0019).
- The DLL exports `MeetBurnerCameraVersion()` (returns the constant) and `DllRegisterServer` writes
  `HKCU\Software\Classes\CLSID\{DBD3A890-…}\MeetBurnerDriverVersion` (DWORD).

### Check on start

- On mount the setup page calls `camera_driver_ensure`, which reports the status plus whether it
  changed anything.
- `driver::ensure` re-registers the bundled DLL only when **all** of these hold: the driver is
  registered, the bundled DLL reports a version, and the stored version is missing or lower.
- So: pre-0020 registrations (no value) are upgraded, older versions are upgraded, equal or newer
  versions are left untouched (never downgrade), and an unregistered driver is **not** registered
  automatically (ADR-0017 keeps that an explicit user action).

### Errors

A failed update is surfaced in the setup page and the app keeps running: the camera snapshot, the
web UI and the rest of the app do not depend on the registration succeeding.

## Consequences

- Existing users get the ADR-0019 fixes on the next launch, with no unregister/register dance.
- Releasing a new filter is: bump `MB_CAMERA_DRIVER_VERSION`, build, ship. No migration code.
- The registry value is the source of truth for the installed version; a DLL file swapped by hand
  behind the app's back is not detected until the next version bump.
- The status line shows the installed version (`… v2`), which makes support questions answerable.
