# 0025. Binaries ship as GitHub Releases built from tags

- Status: accepted
- Date: 2026-10-23
- Related: [0012](0012-deploy-on-push.md) (the web app deploys on push), [0017](0017-app-registers-the-driver.md) (the installer bundles the driver), [0020](0020-driver-version-and-self-update.md) (the app updates the driver on start)

## Context

The desktop app builds NSIS/MSI installers and a portable folder locally (`npm run build:desktop`), but nothing produces them from a clean machine and there is nowhere to download them. M9 asks for a release workflow.

What the pipeline has to respect:

- The installer must contain the DirectShow DLL built from `native/vcam-windows`; Tauri does not build it, and `tools/copy-driver.mjs` only copies an existing build.
- The driver has its own version (`MB_CAMERA_DRIVER_VERSION`, ADR-0020) that has to move together with the app: a release is a pair (app, driver), never one without the other.
- CI already runs the tests on every PR, including CMake + CTest for the filter.
- The Windows certificate is not available yet, so the first releases ship unsigned — but the pipeline should not have to be rewritten when it arrives.
- The repository is private today. Release assets of a private repository require authentication: end users cannot download them, `releases/latest/download/<asset>` does not work, and an in-app updater could not read them either.

## Decision

### The tag is the release

`.github/workflows/release.yml` runs on `push` of a `v*` tag.

- Only a tag publishes. `main` keeps deploying the web app on push (ADR-0012); binaries are never built from an untagged commit.
- The version lives once, in `apps/desktop/src-tauri/tauri.conf.json`. `apps/desktop/package.json` and `apps/desktop/src-tauri/Cargo.toml` must match it, and the tag must equal all three. The `check` job fails on any mismatch: bumping the version is a normal commit before tagging, not something the pipeline does.
- `workflow_dispatch` runs the same jobs without publishing. That is the dry run; it still builds and signs when the secrets exist.
- The tag must point at a commit that went through CI on `main`. The release workflow re-runs `npm test` and `npm run build:web`, and the Windows job rebuilds the driver with CMake, so a tag cannot ship a broken tree.

### Assets

| Asset | Produced by |
|---|---|
| `MeetBurner_<version>_x64-setup.exe` | NSIS bundle (`--bundles nsis`) |
| `MeetBurner_<version>_x64_en-US.msi` | WiX bundle (`--bundles msi`) |
| `MeetBurner_<version>_portable_x64.zip` | `meetburner.exe` + `resources/MeetBurnerCamera.dll`, zipped by the workflow |
| `MeetBurner_<version>_amd64.deb` | Debian bundle on `ubuntu-22.04` |
| `MeetBurner_<version>_amd64.AppImage` | AppImage bundle on `ubuntu-22.04` |
| `SHA256SUMS.txt` | `sha256sum` over the files above |

- The Windows job builds the DLL first, signs it, and only then runs `tauri build`, so the installer and the portable zip both carry the same signed driver. The DLL is a bundle *resource*: Tauri does not sign resources, so it needs its own signtool call.
- Bundle targets are passed explicitly with `--bundles` instead of relying on `bundle.targets` being filtered per platform; a Linux runner never tries to build an MSI.
- Linux builds on `ubuntu-22.04` (not `ubuntu-latest`) because the glibc it links against sets the oldest distribution the `.deb`/AppImage runs on.
- x64 only for now, matching the driver build; ARM64 is a separate decision.

### Draft first, then publish

`publish` runs after both platform jobs, creates the release **as a draft**, uploads every asset plus the checksums, and only then flips it to published with `--draft=false --latest`.

- A failed Windows or Linux job never leaves a half-populated release behind.
- Release notes are GitHub's generated notes (merged PR titles), so there is no changelog file to keep in sync.
- When the build was unsigned, the job appends one line to the notes saying so.

### Signing is wired, not required

Two repository secrets: `WINDOWS_CERTIFICATE` (base64 PFX) and `WINDOWS_CERTIFICATE_PASSWORD`.

- When they exist: the workflow imports the PFX, signs `MeetBurnerCamera.dll` with `signtool`, and passes `certificateThumbprint`, `digestAlgorithm=sha256` and a timestamp URL to `tauri build --config <file>` so Tauri signs the exe and both installers with the same certificate.
- When they do not exist: the same jobs build unsigned artifacts and the release says so.
- Assets are always signed *before* they are uploaded. There is no "sign later" step, so a release never contains a file that differs from what was built.
- `tauri.conf.json` stays free of signing settings: the certificate is injected with `--config`, so a local build is never blocked by a missing certificate.

### This repository is the distribution point

- The workflow publishes to `POLYTROPO-ES/meetburner` with `contents: write`, using the default `GITHUB_TOKEN`. Nothing in the pipeline depends on the repository being public.
- While the repository is private, only collaborators can download the assets. Flipping it to public is a settings change, not a pipeline change: existing releases become downloadable, `releases/latest/download/<asset>` starts working, and later releases need no new run.
- The download button on meetburner.com waits for that flip, so the site never links to an asset that requires a login.
- If the source has to stay private later, the only change is the `publish` job: it uploads the same artifacts to a public releases-only repository. The build jobs are unaffected.

### Deferred

- **In-app updater** (`tauri-plugin-updater`): needs `createUpdaterArtifacts`, a minisign key and a *publicly reachable* `latest.json`. It gets its own ADR once the repository is public. Driver updates stay with ADR-0020, which runs on every start and does not need to download anything.
- **macOS** (M12): adds a job with Developer ID signing and notarization. `publish` already uploads whatever the jobs produce, so only the job and the asset list grow.
- **`tauri-apps/tauri-action`**: not used. It does not build the driver, the portable zip still has to be assembled, and it creates the release before both platforms finish. The explicit steps are shorter to audit than a wrapper plus the missing steps.

## Consequences

- Releasing is: bump the version, open the PR, merge, `git tag vX.Y.Z && git push origin vX.Y.Z`. Nothing is published from a laptop.
- Every release contains the driver the app registers (ADR-0017) and version-checks on start (ADR-0020); there is no separate driver download.
- The release is the permanent record; the same files are also kept as workflow artifacts for 90 days for debugging.
- Until the certificate arrives, releases are unsigned and SmartScreen warns on first run (documented in `apps/desktop/README.md`).
- A tag pushed by mistake produces a draft-first release that a maintainer can delete; no `main` state is affected.
