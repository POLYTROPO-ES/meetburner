# MeetCost

Shows the live cost of a meeting. Available as a web app and as a desktop app (Windows/Linux in stage 1, macOS in stage 2) that publishes the counter as a **virtual webcam**, optionally overlaid on a real webcam.

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
  core/           Pure TS: cost model, Spain salary presets, i18n (en/es)
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
- Desktop: Rust stable + Tauri 2 prerequisites (https://v2.tauri.app/start/prerequisites/)

## Commands

```bash
npm install
npm test            # core unit tests
npm run dev:web     # web app on http://localhost:5173
npm run build:web
npm run dev:desktop # requires Rust
```
