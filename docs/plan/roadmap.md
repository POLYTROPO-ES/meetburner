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
- [x] Polish: responsive down to 360 px, accessibility (names, contrast, keyboard, live region), popup-blocked fallback
- [x] Deploy to Cloudflare Pages (`npm run deploy:web`, `_headers`), served at https://meetburner.com
- **Done when:** a non-technical user can run a meeting counter in < 30 s.

### M3. Desktop shell — `desktop`
- [~] Tauri 2 app loading `apps/web` (dev URL / dist)
- [ ] Counter opens as native `WebviewWindow` (always-on-top option)
- [~] Camera settings page: output resolution done (ADR-0015); Register/Clean webcam buttons done (ADR-0017); camera live while the app is open, errors shown without stopping the app (ADR-0018); source webcam and card placement done (ADR-0021, ADR-0023)
- [x] One configuration surface: the card is placed, resized and turned on the camera frame and the style block edits that same card, so Layout is derived instead of chosen (ADR-0032)
- [x] Only counter or video with a live preview: the mode is an explicit switch, the camera options appear with video, and a preview that cannot open the webcam only shows a message (ADR-0033)
- [x] Studio shell for the setup page: fixed header and footer, collapsible editor column, draggable splitter, the counter always in view (ADR-0034)
- [ ] Compact style table in the editor column so 1280×720 needs no scrolling
- [~] Packaging in CI: Windows NSIS + MSI, portable one-file exe and zip, Linux `.deb` + AppImage (`.github/workflows/release.yml`, ADR-0025, ADR-0026); pending the first green dry run
- **Done when:** installers build in CI for Windows and Linux.

### M4. Frame renderer — `desktop`
- [x] Rust module `camera::renderer`: renders counter (same themes, fonts and sizes as web) into a BGRA buffer 1280x720
- [x] Layout rule: `fullscreen` without source webcam, `widget` over a source webcam (ADR-0006)
- [x] Large amounts: apply `fitCost` / `formatMoneyCompact` with the renderer's text metrics (ADR-0010)
- [x] Tauri commands: `camera_start(config)`, `camera_update(meeting)`, `camera_stop()`
- [x] Fullscreen output resolution selector: 16:9, 9:16, 4:3, 3:4 and 1:1 sizes, applied live (ADR-0015)
- [x] Cost font fills 90 % of the width by default; the size slider multiplies that fit (ADR-0015)
- [x] Live updates: every parameter change reaches the camera on the next frame; only a resolution change re-creates the mapping
- [x] One fixed-capacity mapping for the whole session: a resolution change only changes the header, so meeting apps never lose the device (ADR-0019)
- [x] Bottom-up RGB32 frames and last-frame fallback: upright in Meet/Chrome/ffmpeg and no "Camera not running" flashes (ADR-0019)
- [x] Always on: the camera publishes from app start while the driver is registered, with an idle frame before the meeting starts; failures are shown in the setup page (ADR-0018)
- [~] Snapshot test: structural frame tests (colours, ink per text box, nothing outside the widget card) instead of PNG goldens, because system fonts differ per machine (ADR-0014)
- [ ] Emoji decorations: needs a face with the glyph (system emoji font), currently skipped
- **Done when:** renderer produces 30 fps at < 5% CPU on a mid laptop (not measured yet).

### M5. Linux virtual camera — `vcam-linux`
- [x] Detect an existing `v4l2loopback` device (MeetBurner label, then v4l2loopback's default, then `MEETBURNER_V4L2_DEVICE`); the app reports the `modprobe` command instead of loading kernel modules (ADR-0002, ADR-0030)
- [x] `V4l2Sink` implements `VirtualCameraSink` (YUYV to `/dev/videoN`)
- [x] `camera_v4l2_status` reports whether a device is available
- [ ] Verified in Chrome (Meet), Zoom, OBS, Firefox
- **Done when:** "MeetBurner Camera" selectable and live in Meet on Ubuntu 24.04 and Fedora.

### M6. Windows virtual camera (DirectShow) — `vcam-windows`
- [x] C++ DLL source filter + shared memory reader (`native/vcam-windows/dshow`, ADR-0016)
- [x] `WinSharedMemSink` in Rust (producer), ADR-0013
- [x] Per-user registration + unregister scripts (`native/vcam-windows/scripts`), ADR-0016
- [x] Installer bundles the DLL; the app registers/cleans it with two buttons (ADR-0017)
- [x] Driver version check: an older registration is re-registered with the bundled DLL on start (ADR-0020)
- [x] Verified with ffmpeg (DirectShow) at 1280x720 and 720x1280, 30 fps
- [ ] Verified in Zoom, OBS, Chrome/Edge (Meet), Teams (classic)
- **Done when:** camera works on Windows 10 and 11.

### M7. Webcam overlay — `desktop`
- [x] Source webcam enumeration and capture (`nokhwa`, ADR-0021)
- [x] Compositing: `widget` card over the webcam frame, scaled to the output size (ADR-0021)
- [x] Fallback to counter-only if source camera is busy/unplugged
- [x] Mirror the camera image and the counter independently, H and V (ADR-0022)
- [x] Move and resize the card by dragging it on a preview, with a corner handle (ADR-0023)
- [x] Recover the camera when the app was killed while a meeting app kept the mapping open (ADR-0024)
- **Done when:** overlay works on both Linux and Windows sinks.

### M8. Windows virtual camera (Media Foundation) — `vcam-windows`
- [x] COM media source DLL (`native/vcam-windows/mf`) reading the same shared memory
- [x] `MFCreateVirtualCamera` lifecycle owned by the app (Windows 11 only)
- [ ] Verified in new Teams and Windows Camera app
- [ ] Embed `MeetBurnerMfCamera.dll` in the single-file portable exe (ADR-0026 pattern) so the one-file download can offer the Windows 11 camera too; today the zip and the installers carry it
- **Done when:** MF camera visible on Windows 11 without breaking DirectShow one.

### M9. Stage 1 release — `architect` + `desktop`
- [x] Release workflow: tag `vX.Y.Z` builds Windows (NSIS, MSI, portable exe and zip), Linux (`.deb`, AppImage) and macOS (`.dmg`) and publishes a draft-first GitHub Release with `SHA256SUMS.txt` (ADR-0025, ADR-0026, ADR-0028); first release: `v0.1.0`
- [x] Release notes: GitHub's generated notes from the merged PRs, plus the portable-build and unsigned-build notes the workflow appends
- [x] MIT license and the [code signing policy](../code-signing-policy.md) the SignPath review asks for (ADR-0027)
- [ ] Code signing (Windows): SignPath Foundation application pending (ADR-0027); until it is approved, releases ship unsigned and the workflow has no certificate to import
- [x] Flip the repository to public so `releases/latest/download/…` works for end users, then add the download button on meetburner.com
- [x] User docs: install, Linux module setup, troubleshooting — [user guide](../user-guide.md)

## Stage 2 — macOS

### M10. macOS app — `desktop`
- [x] Universal (arm64 + x64) `.dmg` built in CI and attached to the release, without the virtual camera (ADR-0028)
- [ ] Developer ID signing and notarization in CI (needs the Apple Developer Program)

### M11. macOS virtual camera — `vcam-macos`
- [ ] Swift CoreMediaIO Camera Extension embedded in the bundle
- [ ] Activation flow (`OSSystemExtensionRequest`) + user approval UX
- [ ] `CmioSink` in Rust (via Swift bridge) pushing frames to the sink stream
- [ ] Verified in Zoom, Meet (Chrome/Safari), Teams, FaceTime

### M12. macOS overlay + release — `desktop` + `vcam-macos`
- [ ] AVFoundation capture via `nokhwa`, camera permission prompts
- [ ] DMG release
