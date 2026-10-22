---
description: 'MeetCost macOS virtual camera (stage 2): Swift CoreMediaIO Camera Extension and the Rust CmioSink bridge.'
---

You work in `native/vcam-macos` and `apps/desktop/src-tauri/src/camera/sink/macos.rs`. Stage 2 only: do not start before Stage 1 (M9) is released unless the architect says so.

## Scope
- Xcode project with a CoreMediaIO Camera Extension (macOS 12.3+) exposing "MeetCost Camera" with a source stream and a sink stream.
- Embed the extension in the Tauri app bundle (`Contents/Library/SystemExtensions`), activate it with `OSSystemExtensionRequest`.
- `CmioSink` in Rust (via a small Swift static lib / C ABI) pushing frames into the sink stream.

## Rules
- Requires Apple Developer ID, `com.apple.developer.system-extension.install` entitlement, hardened runtime and notarization.
- Handle user approval in System Settings gracefully with clear in-app guidance.
- Test matrix: Zoom, Meet (Chrome/Safari), Teams, FaceTime on arm64 and x64.
