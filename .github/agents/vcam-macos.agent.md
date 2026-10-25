---
description: 'MeetBurner macOS virtual camera (stage 2): Swift CoreMediaIO Camera Extension and the Rust CmioSink bridge.'
---

You work in `native/vcam-macos` and `apps/desktop/src-tauri/src/camera/sink/macos.rs` (does not exist yet). Stage 2, **blocked**: the macOS app already ships as an unsigned universal `.dmg` without the virtual camera (ADR-0028). Do not start M11 until the Apple Developer Program account exists and the architect supersedes ADR-0028.

## Scope
- Xcode project with a CoreMediaIO Camera Extension (macOS 12.3+) exposing "MeetBurner Camera" with a source stream and a sink stream.
- Embed the extension in the Tauri app bundle (`Contents/Library/SystemExtensions`), activate it with `OSSystemExtensionRequest`.
- `CmioSink` in Rust (via a small Swift static lib / C ABI) pushing frames into the sink stream; it replaces `NullSink` on macOS in `camera/engine.rs`.
- When the sink exists, `hasVirtualCamera()` in `apps/web/src/platform.ts` must include macOS (with the `web` agent).

## Rules
- TDD (ADR-0008): the Rust side of the bridge (frame handoff, pixel format) is unit-tested first with `cargo test`.
- Requires Apple Developer ID, `com.apple.developer.system-extension.install` entitlement, hardened runtime and notarization (M10 signing lands first).
- Same frame contract as the other sinks: the producer renders top-down BGRA; the sink converts, never the renderer.
- Handle user approval in System Settings gracefully with clear in-app guidance.
- Test matrix: Zoom, Meet (Chrome/Safari), Teams, FaceTime on arm64 and x64.
