# Roadmap (step by step)

Status legend: `[x]` done, `[~]` scaffolded, `[ ]` todo. Each milestone lists the owning agent (see `.github/agents/`).

## Stage 1 — Web + Windows + Linux

### M0. Foundations — `architect`
- [x] Monorepo layout, README, ADRs, roadmap, agent files
- [ ] CI (GitHub Actions): `npm ci && npm test && npm run build:web` on every PR
- [ ] CI: `cargo fmt --check`, `cargo clippy`, `cargo test` in `apps/desktop/src-tauri`

### M1. Core model — `core`
- [~] `hourlyRate`, `meetingSnapshot`, formatting helpers
- [~] Spain salary presets (EUR/year) and configurable hours/year (default 1800)
- [~] i18n dictionary en/es
- [~] Unit tests (Vitest)
- **Done when:** 100% of core functions covered, no runtime dependencies.

### M2. Web app — `web`
- [~] Setup page: currency, hours/year, start time, participants (preset or custom)
- [~] Start opens counter popup; fluid counter (rAF) with total cost, elapsed, person-time, cost/min
- [~] Persistence in `localStorage`, live sync between windows (`storage` event)
- [ ] Polish: responsive, dark theme, accessibility (labels, contrast), popup-blocked fallback
- [ ] Deploy to static hosting
- **Done when:** a non-technical user can run a meeting counter in < 30 s.

### M3. Desktop shell — `desktop`
- [~] Tauri 2 app loading `apps/web` (dev URL / dist)
- [ ] Counter opens as native `WebviewWindow` (always-on-top option)
- [ ] Settings page for camera (mode, source webcam, overlay layout)
- [ ] Packaging: Windows NSIS/MSI, Linux `.deb` + AppImage
- **Done when:** installers build in CI for Windows and Linux.

### M4. Frame renderer — `desktop`
- [ ] Rust module `camera::renderer`: renders counter (same look as web) into a BGRA buffer 1280x720
- [ ] Tauri commands: `camera_start(config)`, `camera_update(meeting)`, `camera_stop()`
- [ ] Snapshot test (PNG golden file)
- **Done when:** renderer produces 30 fps at < 5% CPU on a mid laptop.

### M5. Linux virtual camera — `vcam-linux`
- [ ] Detect `v4l2loopback`; one-click setup via `pkexec modprobe ...`
- [ ] `V4l2Sink` implements `VirtualCameraSink` (YUYV)
- [ ] Verified in Chrome (Meet), Zoom, OBS, Firefox
- **Done when:** "MeetCost Camera" selectable and live in Meet on Ubuntu 24.04 and Fedora.

### M6. Windows virtual camera (DirectShow) — `vcam-windows`
- [ ] C++ DLL source filter + shared memory reader (`native/vcam-windows/dshow`)
- [ ] `WinSharedMemSink` in Rust (producer)
- [ ] Installer registers/unregisters the DLL (x64)
- [ ] Verified in Zoom, OBS, Chrome/Edge (Meet), Teams (classic)
- **Done when:** camera works on Windows 10 and 11.

### M7. Webcam overlay — `desktop`
- [ ] Source webcam enumeration and capture (`nokhwa`)
- [ ] Compositing: position, scale, opacity; mirror option
- [ ] Fallback to counter-only if source camera is busy/unplugged
- **Done when:** overlay works on both Linux and Windows sinks.

### M8. Windows virtual camera (Media Foundation) — `vcam-windows`
- [ ] COM media source DLL (`native/vcam-windows/mf`) reading the same shared memory
- [ ] `MFCreateVirtualCamera` lifecycle owned by the app (Windows 11 only)
- [ ] Verified in new Teams and Windows Camera app
- **Done when:** MF camera visible on Windows 11 without breaking DirectShow one.

### M9. Stage 1 release — `architect` + `desktop`
- [ ] Code signing (Windows), release workflow, changelog
- [ ] User docs: install, Linux module setup, troubleshooting

## Stage 2 — macOS

### M10. macOS app — `desktop`
- [ ] Universal build (arm64 + x64), Developer ID signing, notarization in CI

### M11. macOS virtual camera — `vcam-macos`
- [ ] Swift CoreMediaIO Camera Extension embedded in the bundle
- [ ] Activation flow (`OSSystemExtensionRequest`) + user approval UX
- [ ] `CmioSink` in Rust (via Swift bridge) pushing frames to the sink stream
- [ ] Verified in Zoom, Meet (Chrome/Safari), Teams, FaceTime

### M12. macOS overlay + release — `desktop` + `vcam-macos`
- [ ] AVFoundation capture via `nokhwa`, camera permission prompts
- [ ] DMG release
