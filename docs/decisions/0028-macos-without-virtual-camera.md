# 0028. macOS ships without the virtual camera until the Apple account exists

- Status: accepted
- Date: 2026-10-23

## Context

Stage 2 (M10-M12) adds macOS. The app itself turned out to be cross-platform already: the
pipeline falls back to `NullSink` on every target that is not Windows
(`camera/engine.rs`), `build.rs` embeds no driver there, and the Linux job has been bundling
the same shell for a while. Shipping a macOS build is therefore a CI job, not a port.

The virtual camera is the exception. On macOS it has to be a CoreMediaIO Camera Extension,
which is a *system extension*: macOS loads it only when it is Developer ID signed and
notarized. That needs the Apple Developer Program, and the extension cannot be activated
from a hosted runner — approval is an interactive step on a real Mac.

## Decision

- The release workflow builds a universal (arm64 + x64) `.dmg` on `macos-latest` and attaches
  it to the release, unsigned.
- The setup page shows the camera section only where a sink exists: `hasVirtualCamera()` in
  `apps/web/src/platform.ts` is true on the Windows desktop build and false everywhere else,
  and no camera command is invoked on the other platforms.
- The per-platform scope is documented in `README.md` instead of promising parity.
- This ADR is superseded when the Apple Developer account exists: signing, notarization and
  the CMIO extension then land in M10/M11.

## Consequences

- The macOS download is usable for the counter and share links, but Gatekeeper blocks the
  first launch until the user right-clicks the app and chooses *Open*. The release notes say
  so, in the same voice as the unsigned Windows note.
- macOS users cannot put the counter inside a meeting until M11, and the "verified in
  Zoom/Meet/Teams/FaceTime" claim stays Windows-only.
- Linux loses the camera section too, which is correct: `native/vcam-linux` has no sink yet
  (M5). It reappears there when the v4l2loopback sink exists.
