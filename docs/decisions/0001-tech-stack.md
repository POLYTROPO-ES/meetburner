# 0001. Tech stack

- Status: accepted
- Date: 2026-10-22

## Context

We need a web app and desktop apps (Windows/Linux first, macOS later) sharing the same UI. The desktop app must drive OS-level virtual cameras, which requires native code.

## Decision

| Area | Choice | Why |
|------|--------|-----|
| Monorepo | npm workspaces | Zero extra tooling, Node is already required |
| Shared logic | `packages/core` in TypeScript, no runtime deps | Pure, unit-testable, used by web and desktop UI |
| UI | React + TypeScript + Vite | Mainstream, fast dev loop, same bundle for web and Tauri |
| Desktop shell | Tauri 2 | Small binaries, Rust backend is a natural fit for frame rendering and OS camera APIs |
| Frame rendering | Rust: `tiny-skia` + `ab_glyph` | CPU 2D raster, no GPU/window dependency |
| Webcam capture | Rust: `nokhwa` | Cross-platform (MSMF / V4L2 / AVFoundation) |
| Windows vcam | C++ DirectShow source filter DLL, then Media Foundation virtual camera | See ADR-0002 |
| Linux vcam | `v4l2loopback` + Rust `v4l` crate | De facto standard (used by OBS) |
| macOS vcam (stage 2) | Swift CoreMediaIO Camera Extension | Only supported modern API |
| Tests | Vitest (TS), `cargo test` (Rust) | |
| Web hosting | Any static host (GitHub Pages / Cloudflare Pages) | Web app is fully static |

## Consequences

- Contributors need Node; desktop contributors also need Rust and a C++ toolchain (MSVC on Windows).
- Web UI must avoid browser-only APIs without a Tauri fallback (e.g. `window.open` vs `WebviewWindow`).
