# MeetBurner

<img src="docs/meetburner-favicon.svg" alt="MeetBurner logo" width="96" />

**Every meeting burns money.**

## See the cost. Feel the burn.

A live meeting-cost overlay that shows the combined cost of everyone's time. Available as a web app and as a desktop app (Windows/Linux in stage 1, macOS in stage 2) that publishes the counter as a **virtual webcam**, optionally overlaid on a real webcam.

- Product scope: [mvp.md](mvp.md)
- Architecture: [docs/architecture.md](docs/architecture.md)
- Decisions (ADRs): [docs/decisions/](docs/decisions/)
- Roadmap / step-by-step plan: [docs/plan/roadmap.md](docs/plan/roadmap.md)
- Agents: [.github/agents/](.github/agents/)

## Layout

```
apps/
  web/            React + TS + Vite UI (also the desktop frontend)
  desktop/        Tauri 2 shell (src-tauri = Rust)
packages/
  core/           Pure TS: cost model, countries, salary presets, themes, i18n (en/es)
native/
  vcam-windows/   DirectShow filter (stage 1) + Media Foundation vcam (stage 1b)
  vcam-linux/     v4l2loopback setup + notes
  vcam-macos/     CoreMediaIO Camera Extension (stage 2)
docs/
  architecture.md
  decisions/      ADRs (one file per decision)
  plan/           roadmap and milestone notes
```

## Requirements

- Node.js 20+ (npm workspaces)
- Desktop: Rust stable (rustup) + Tauri 2 prerequisites (https://v2.tauri.app/start/prerequisites/): MSVC C++ build tools and WebView2 on Windows
- Windows virtual camera: CMake (the one bundled with Visual Studio 2022 works: use a *Developer PowerShell for VS 2022*)

## Development workflow

TDD red/green/refactor ([ADR-0008](docs/decisions/0008-tdd.md)): write a failing test, make it pass, then refactor.

## Commands

```bash
npm install
npm test            # core unit tests
npm run dev:web     # web app on http://localhost:5173
npm run build:web
npm run deploy:web  # Cloudflare Pages (needs `npx wrangler login` or CLOUDFLARE_API_TOKEN)
npm run dev:desktop # requires Rust
npm run build:desktop # NSIS/MSI installers + portable exe (Windows)
```

## Desktop builds

- **Installer** (recommended): `MeetBurner_<version>_x64-setup.exe` or `.msi`, from `apps/desktop/src-tauri/target/release/bundle`.
- **Portable**: `meetburner.exe` plus `resources\MeetBurnerCamera.dll`, no installation; register the webcam from the app once per Windows account.

Step-by-step, including what each file is for: [apps/desktop/README.md](apps/desktop/README.md#install-vs-portable).
