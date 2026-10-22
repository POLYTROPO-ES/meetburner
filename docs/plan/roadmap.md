# Roadmap (step by step)

Status legend: `[x]` done, `[~]` scaffolded, `[ ]` todo. Each milestone lists the owning agent (see `.github/agents/`).

## Stage 1 — Web + Windows + Linux

### M0. Foundations — `architect`
- [x] Monorepo layout, README, ADRs, roadmap, agent files
- [x] Local toolchain: Node 24, Rust (rustup), MSVC (VS 2022), WebView2, CMake (bundled with VS 2022)
- [x] Local git repo with per-file commits
- [x] CI (GitHub Actions): `npm ci && npm test && npm run build:web` on every PR
- [x] CI: `cargo fmt --check`, `cargo clippy`, `cargo test` in `apps/desktop/src-tauri`
- [x] CD: deploy `main` to Cloudflare Pages on every push (`.github/workflows/deploy-web.yml`, ADR-0012)

### M1. Core model — `core` (TDD, ADR-0008)
- [x] `hourlyRate`, `meetingSnapshot`, formatting helpers
- [x] Configurable hours/year (default 1800 in Spain)
- [x] Countries (ES, PT, FR, DE, IT, NL, GB, US, MX) with currency, hours/year, employer contribution rate (ADR-0005)
- [x] Total (employer) cost on by default, can be switched off
- [x] Role salary presets per country (rough estimates)
- [x] Themes + display settings as shared data (ADR-0006)
- [x] i18n dictionary en/es
- [x] Unit tests (Vitest)
- [ ] Replace rough salary estimates with official sources (INE, Eurostat, ONS, BLS, INEGI…), keeping source and date per value
- [ ] Southern-hemisphere option for the "4 seasons" style
- **Done when:** 100% of core functions covered, no runtime dependencies.

### M2. Web app — `web`
- [x] Setup page: country, currency, hours/year, total cost, start time, participants (preset or custom)
- [x] Start opens counter popup; fluid counter (rAF) with total cost, elapsed, person-time, cost/min
- [x] Persistence in `localStorage`, live sync between windows (`storage` event)
- [x] Counter styles: classic, modern, formal, halloween, christmas, 4 seasons, minimalist
- [x] Layouts fullscreen/widget; user picks font and size for the cost and the time; live preview
- [x] "Share the cost" link with a meeting summary: total time, total cost, cost/min (ADR-0009)
- [x] Large amounts that do not fit: abbreviate (K, M, B) or shrink the font, user selectable (ADR-0010)
- [ ] Polish: responsive, accessibility (labels, contrast), popup-blocked fallback
- [x] Deploy to Cloudflare Pages (`npm run deploy:web`, `_headers`), served at https://meetburner.com
- **Done when:** a non-technical user can run a meeting counter in < 30 s.

### M3. Desktop shell — `desktop`
- [~] Tauri 2 app loading `apps/web` (dev URL / dist)
- [ ] Counter opens as native `WebviewWindow` (always-on-top option)
- [~] Camera settings page: enable + output resolution (fullscreen) done (ADR-0015); mode, source webcam and overlay layout still pending
- [ ] Packaging: Windows NSIS/MSI, Linux `.deb` + AppImage
- **Done when:** installers build in CI for Windows and Linux.

### M4. Frame renderer — `desktop`
- [x] Rust module `camera::renderer`: renders counter (same themes, fonts and sizes as web) into a BGRA buffer 1280x720
- [x] Layout rule: `fullscreen` without source webcam, `widget` over a source webcam (ADR-0006)
- [x] Large amounts: apply `fitCost` / `formatMoneyCompact` with the renderer's text metrics (ADR-0010)
- [x] Tauri commands: `camera_start(config)`, `camera_update(meeting)`, `camera_stop()`
- [x] Fullscreen output resolution selector: 16:9, 9:16, 4:3, 3:4 and 1:1 sizes, applied live (ADR-0015)
- [x] Cost font fills 90 % of the width by default; the size slider multiplies that fit (ADR-0015)
- [x] Live updates: every parameter change reaches the camera on the next frame; only a resolution change re-creates the mapping
- [~] Snapshot test: structural frame tests (colours, ink per text box, nothing outside the widget card) instead of PNG goldens, because system fonts differ per machine (ADR-0014)
- [ ] Emoji decorations: needs a face with the glyph (system emoji font), currently skipped
- **Done when:** renderer produces 30 fps at < 5% CPU on a mid laptop (not measured yet).

### M5. Linux virtual camera — `vcam-linux`
- [ ] Detect `v4l2loopback`; one-click setup via `pkexec modprobe ...`
- [ ] `V4l2Sink` implements `VirtualCameraSink` (YUYV)
- [ ] Verified in Chrome (Meet), Zoom, OBS, Firefox
- **Done when:** "MeetBurner Camera" selectable and live in Meet on Ubuntu 24.04 and Fedora.

### M6. Windows virtual camera (DirectShow) — `vcam-windows`
- [x] C++ DLL source filter + shared memory reader (`native/vcam-windows/dshow`, ADR-0016)
- [x] `WinSharedMemSink` in Rust (producer), ADR-0013
- [x] Per-user registration + unregister scripts (`native/vcam-windows/scripts`), ADR-0016
- [ ] Installer registers/unregisters the DLL (x64)
- [x] Verified with ffmpeg (DirectShow) at 1280x720 and 720x1280, 30 fps
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
- [ ] Code signing (Windows): certificate pending (owner is getting one); release workflow, changelog
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
