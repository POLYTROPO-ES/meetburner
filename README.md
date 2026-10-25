# MeetBurner

<img src="docs/meetburner-favicon.svg" alt="MeetBurner logo" width="96" />

**Every meeting burns money.**

## See the cost. Feel the burn.

A live meeting-cost overlay that shows the combined cost of everyone's time. Available as a web app and as a desktop app (Windows/Linux in stage 1, macOS in stage 2) that publishes the counter as a **virtual webcam**, optionally overlaid on a real webcam.

- Product scope: [mvp.md](mvp.md)
- Architecture: [docs/architecture.md](docs/architecture.md)
- Decisions (ADRs): [docs/decisions/](docs/decisions/)
- Roadmap / step-by-step plan: [docs/plan/roadmap.md](docs/plan/roadmap.md)
- User guide: [docs/user-guide.md](docs/user-guide.md)
- Code signing policy: [docs/code-signing-policy.md](docs/code-signing-policy.md)
- License: [MIT](LICENSE)
- Agents: [.github/agents/](.github/agents/)

## What each version can do

| Platform | Scope |
|---|---|
| **Windows** desktop | Full: setup page, counter window, webcam overlay and the **MeetBurner Camera** virtual camera — DirectShow, plus the **MeetBurner Camera (Windows 11)** Media Foundation camera for new Teams and Windows Camera ([ADR-0031](docs/decisions/0031-windows-mf-virtual-camera.md)) |
| **Linux** desktop | Everything on the setup page, the counter window, share links and the *MeetBurner Camera* virtual camera once `v4l2loopback` is loaded ([setup](native/vcam-linux/README.md), [ADR-0030](docs/decisions/0030-linux-camera-v4l2loopback.md)) |
| **macOS** desktop | Setup page, counter window and share links. No virtual camera yet: a CoreMediaIO camera extension needs Apple Developer signing and notarization (M11, [ADR-0028](docs/decisions/0028-macos-without-virtual-camera.md)) |
| **Web** ([meetburner.com](https://meetburner.com)) | Full web feature set: setup, counter window, share links. A browser cannot publish a virtual camera |

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

A release is cut from a tag: `git tag vX.Y.Z && git push origin vX.Y.Z` builds Windows and Linux on GitHub Actions and publishes a draft-first release with checksums ([ADR-0025](docs/decisions/0025-github-releases.md)). `workflow_dispatch` on `.github/workflows/release.yml` is the dry run.

| Download | What it is |
|---|---|
| `MeetBurner_<version>_x64-setup.exe`, `..._en-US.msi` | installer, recommended |
| `MeetBurner_<version>_x64.exe` | portable, one file: the camera driver is inside and is written next to the exe on the first registration ([ADR-0026](docs/decisions/0026-portable-single-file.md)) |
| `MeetBurner_<version>_portable_x64.zip` | portable, the same exe plus `resources\MeetBurnerCamera.dll`; use it when antivirus or SmartScreen flags the one-file build |
| `..._amd64.deb`, `..._amd64.AppImage` | Linux |
| `MeetBurner_<version>_universal.dmg` | macOS, Apple silicon and Intel. Unsigned: Gatekeeper asks for confirmation on the first launch (ADR-0028) |

Release assets are public and the footer of [meetburner.com](https://meetburner.com) links to the latest one.

Locally, `npm run build:desktop` writes the NSIS/MSI installers to `apps/desktop/src-tauri/target/release/bundle` and the exe to `target/release`. Step-by-step, including what each file is for: [apps/desktop/README.md](apps/desktop/README.md#install-vs-portable).

## AI usage

MeetBurner was built with AI coding agents: GitHub Copilot Chat in VS Code, agent mode, with the custom
agents in [.github/agents/](.github/agents/). From the first commit to v0.3.3 that took five chat
sessions (22–25 October 2026), 115 chat requests and about 1,700 model calls.

| | Claude Opus 5.5 | DeepSeek V4.1 Flash |
|---|---|---|
| Access | GitHub Copilot | own DeepSeek API key, added to Copilot as a custom model |
| Sessions · chat requests · model calls | 2 · 23 · 327 | 4 · 92 · 1,378 |
| Input tokens | ~84 M, about 93 % read from the prompt cache | ~346 M, about 85–93 % cache hits |
| Output tokens | 0.52 M | 1.16 M |
| Billed | **~$30** (GitHub) | **~$10** (DeepSeek) |
| Estimated from the logs | $33–59; Copilot recorded 5,366 credits (≈ $54 at list price) | $2–6 |

**In total: ~430 M input tokens (mostly cached), 1.7 M output tokens, about $40 billed.** DeepSeek
V4 Pro was configured but never used.

| Session | Dates | Agent | Model | Requests |
|---|---|---|---|---|
| MVP review and first build | 22 Oct | Agent (default) | Opus 5.5 | 13 |
| Windows companion app, virtual camera | 22–23 Oct | `desktop` | DeepSeek Flash | 29 |
| Windows app build | 22 Oct | `desktop` | DeepSeek Flash | 2 |
| GitHub releases for binaries | 23 Oct | `architect` | DeepSeek Flash | 6 |
| Security check, public release, v0.2–v0.3.3 | 23–25 Oct | `architect` | Opus 5.5 + DeepSeek Flash | 10 + 55 |

How the numbers were obtained: VS Code keeps every chat request in its local session store with the
model, the output tokens and the size of the last prompt. Each agent request makes several model calls
(one per tool round) and re-sends the conversation each time, so the total input is reconstructed from
the prompt sizes and the number of calls. Costs use each provider's list prices (Copilot's model table;
[DeepSeek pricing](https://api-docs.deepseek.com/quick_start/pricing), peak and off-peak) under a
cold-cache and a warm-cache assumption. The billed amounts come from the GitHub and DeepSeek dashboards
and are the reference: DeepSeek's bill is higher than the estimate because background calls (such as
conversation summaries) and cache misses are not in the logs, and the Copilot bill is lower than its
credits at list price.

## License

MIT — see [LICENSE](LICENSE). Binaries are code signed through SignPath Foundation once the
application is approved; until then releases are unsigned, as stated in the
[code signing policy](docs/code-signing-policy.md) ([ADR-0027](docs/decisions/0027-public-source-and-signing.md)).
