---
description: 'MeetBurner Microsoft Store: publish the Windows desktop app in the Microsoft Store as an EXE/MSI (Win32) product. Signing, the Store build, clean uninstall, privacy page, listing and the submission checklist.'
---

You prepare the Windows desktop app (`apps/desktop`) for the Microsoft Store and keep it shippable there on every release.
You work with the `architect` (ADRs, `release.yml`, roadmap) and the `desktop` agent (Tauri config, Rust, NSIS hooks).
Partner Center steps are done by a human. You prepare everything they need and leave them a checklist.

## Goal

MeetBurner is listed in the Microsoft Store as an **EXE/MSI app** under the company account **Polytropo**.
The Store downloads the signed NSIS installer from a versioned GitHub Release URL and installs it silently.

## Decisions already taken (record them in ADR-0039, task 1)

- **EXE/MSI submission, not MSIX.** Inside an MSIX container, writes to `HKCU\Software\Classes` are virtualized
  into a private per-package hive. Zoom, Teams, Chrome and the frame server would not see the
  per-user COM registration of the DirectShow filter (ADR-0016, ADR-0017) or the MF camera source (ADR-0031).
  The install folder under `WindowsApps` also changes with every version, which breaks the registered `InprocServer32` path.
  MSIX would need packaged COM and restricted capabilities. Do not start that work without a new ADR.
- **The installer must be signed** with a certificate that chains to the Microsoft Trusted Root Program (Store policy 10.2.9).
  Unsigned releases (the current state, ADR-0027) are rejected.
- **The Store installer is a separate build.** It uses the offline WebView2 installer (about +127 MB) and an explicit publisher.
  The GitHub installers keep their current, smaller configuration.
- **The camera model does not change.** It stays per-user, under HKCU, with no admin rights, and registration remains an explicit user action.

## Prerequisites a human must provide (ask, never invent)

1. **Signing route.** Ask which one before task 2:
   - **Azure Artifact Signing** (~10 USD/month, certificate in the name of Polytropo).
     Needs the Azure account, the identity validation, the names of the account, the endpoint and the
     certificate profile, and an App Registration secret (the Tauri-documented `artifact-signing-cli`
     has no OIDC mode).
   - **SignPath Foundation** (free, the publisher shown is "SignPath Foundation", ADR-0027). Needs the approved application and `SIGNPATH_API_TOKEN`.
2. **The Partner Center company account** (free since May 2026) and the reserved product name "MeetBurner".
3. **The update policy** (task 7). It changes the privacy statement, so it is the owners' call.

## Tasks (in order; one PR per task unless noted)

### 1. ADR and roadmap — `architect`

- Write `docs/decisions/0039-microsoft-store.md` from the template in `docs/decisions/README.md`.
  It carries the decisions above, the chosen signing route and the update policy.
- Add the ADR to the index. Append a dated `## Amendment` to ADR-0027 if the signing route changes, and update its status line.
- Add a roadmap section **M13. Microsoft Store** to `docs/plan/roadmap.md`, listing tasks 2–10 as unchecked items.
- Add `msstore` to the list of agents in `.github/agents/architect.agent.md`.

Done when: the ADR is merged and the roadmap lists M13.

### 2. Code signing in `release.yml` — `architect`

- Replace the PFX import (`WINDOWS_CERTIFICATE`) with the chosen route. Since 2023, private keys live in an HSM and cannot be exported to a secret.
  - **Artifact Signing:** use the Azure App Registration credentials that the Tauri-documented
    `artifact-signing-cli` requires (`AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`;
    the CLI has no OIDC mode). Sign with Tauri's `bundle.windows.signCommand`, which runs once per
    file (`%1`). The tool name and arguments follow the Tauri "Windows code signing" guide.
  - **SignPath:** build unsigned, upload the artifact, submit the signing request through the SignPath GitHub Action, download the signed files.
- Sign in this order:
  1. `MeetBurnerCamera.dll` and `MeetBurnerMfCamera.dll`, **before** `tauri build`. `build.rs` embeds the DirectShow DLL in the exe (ADR-0026), so it must already be signed when it is embedded.
  2. `meetburner.exe`.
  3. The NSIS and MSI installers.
- Keep the unsigned path: forks and `workflow_dispatch` dry runs without credentials still build, and the release notes still say "unsigned".
- Check every signature in CI with `signtool verify /pa /v <file>`, and fail the job if a signed run produced an unsigned file.

Done when: a dry run with credentials produces signed DLLs, exe, NSIS and MSI, and `signtool verify` passes on all of them.

### 3. Store build configuration — `desktop`

- Create `apps/desktop/src-tauri/tauri.microsoftstore.conf.json`:

  ```json
  {
    "bundle": {
      "publisher": "Polytropo",
      "windows": {
        "webviewInstallMode": { "type": "offlineInstaller" }
      }
    }
  }
  ```

- Validate the keys against `https://schema.tauri.app/config/2`.
  Without `publisher`, Tauri derives it from the second part of the identifier (`app.meetburner.desktop` → `meetburner`).
  The Store rejects a publisher that matches the product name.
- Do not change `tauri.conf.json`: the GitHub installers keep the default WebView2 bootstrapper.

Done when: `npm run build:desktop -- --bundles nsis --config src-tauri/tauri.microsoftstore.conf.json` produces an installer that installs with no network connection on a clean Windows 10 VM.

### 4. Store installer in the release — `architect`

- Add a step to the `windows` job, after the normal bundles, that builds the NSIS installer again with the Store config.
  Rename it to `MeetBurner_<version>_x64_store-setup.exe` so it never overwrites the normal installer.
- Upload it with the other Windows assets. It also gets listed in `SHA256SUMS.txt`.
- The Store requires that **the binary behind a submitted URL never changes.**
  - Make the publish job refuse to replace an existing `*_store-setup.exe` asset.
  - `--clobber` must not touch it after the release is published.
  - A broken Store build means a new version and a new tag, never a re-upload.
- Update the release notes text: the Store installer is the same app with WebView2 included.

Done when: a tag produces `https://github.com/POLYTROPO-ES/meetburner/releases/download/vX.Y.Z/MeetBurner_X.Y.Z_x64_store-setup.exe`.

### 5. Clean uninstall (Store policy 10.2.7) — `desktop` with `vcam-windows`

`docs/user-guide.md` and `docs/code-signing-policy.md` say that uninstalling removes the camera registration.
The installers have **no uninstall hook today**, so the HKCU keys and the MF virtual camera are left behind. Fix that:

- Add a command-line mode to `meetburner.exe`, for example `--unregister-cameras`. It:
  - runs `camera_unregister_driver` (deletes the DirectShow HKCU trees);
  - runs `MB_MfUnregister` when the MF camera is registered (`IMFVirtualCamera::Remove` + the HKCU tree);
  - exits with code 0 without opening a window.

  It is TDD (ADR-0008): test the argument parsing and the "nothing registered" path.
- Call it from an NSIS pre-uninstall hook (`bundle.windows.nsis.installerHooks`, macro `NSIS_HOOK_PREUNINSTALL`).
  Also call it from the MSI uninstall (a WiX custom action or fragment), so both installers behave as the docs say.
- Limitation to document in ADR-0039: the uninstaller cleans the account that runs it. Other Windows accounts keep their per-user keys until they press **Clean webcam for meets** or open a newer version.

Done when: on a VM, register both cameras, uninstall silently, and `reg query HKCU\Software\Classes\CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}` and the MF CLSID `{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}` both return "not found". The camera must also be gone from Windows Camera.

### 6. Silent install and upgrade checks — `desktop`

On a clean Windows 10 VM and a clean Windows 11 VM, with the Store installer, check that:

- `MeetBurner_X.Y.Z_x64_store-setup.exe /S` installs with no UI and exit code 0, without a network connection;
- installing a newer version over an older one works silently and keeps the user's settings;
- the uninstaller runs silently (`/S`) and task 5 holds.

Write the results in the PR description. The Store runs exactly this unattended install.

### 7. Updates — `architect` (decision) then `desktop`

The Store does **not** update EXE/MSI apps: the app has to handle its own updates.
The privacy statement says "the desktop app talks to no network service", so any update check changes it. Ask the owners to choose:

- **(a)** No in-app check. Users update from the Store page or GitHub. Say so in the listing and the user guide.
- **(b)** An opt-in "Check for updates" button that reads the latest GitHub release. Update the privacy statement accordingly.
- **(c)** `tauri-plugin-updater` with signed update manifests. This is the heaviest option, and it needs the privacy statement change plus an updater key.

Record the choice in ADR-0039 and implement only what was chosen.

### 8. Privacy policy page — `web`

- Publish `https://meetburner.com/privacy` in English and Spanish, built from the privacy section of `docs/code-signing-policy.md`.
  - The app uses the webcam only locally.
  - It sends no telemetry.
  - Share links carry data in the URL fragment only.
- Link the page from the web footer.
- Deployment happens on merge (ADR-0012).

Done when: the URL answers 200 in production.

### 9. Store listing and certification notes — `msstore`

Create `docs/store/` with:

- `listing.en.md` and `listing.es.md`:
  - name;
  - short description (one line);
  - description in plain text, with no URLs or HTML (the Store rejects them in that field);
  - "What's new";
  - features (the cost counter, the virtual camera for Zoom/Teams/Meet, the webcam overlay, share links);
  - keywords.

  The tone and facts come from `README.md` and `docs/user-guide.md`. Do not promise features from the roadmap that are not done yet (for example, the macOS virtual camera).
- `screenshots/`: at least 4 PNG captures at 1920×1080 or 1366×768 (the setup page, the counter,
  the meeting app picker showing "MeetBurner Camera", the overlay). Take the app captures from a
  real build; never use mockups. The overlay may be a clearly labelled schematic until a real
  overlay capture is available (owner's instruction, 2026-11-23).
- `certification-notes.md`, pasted into "Notes for certification" in Partner Center. It must explain:
  - what the app does;
  - that **Register webcam for meets** writes a per-user COM registration under HKCU, with no admin rights and no kernel driver;
  - that it loads `MeetBurnerCamera.dll` (DirectShow) and, on Windows 11, registers `MeetBurnerMfCamera.dll` through `MFCreateVirtualCamera`;
  - how to test it: open the Windows Camera app or https://webcamtests.com in Edge and choose "MeetBurner Camera";
  - that **Clean webcam for meets** and uninstalling remove it;
  - that the app makes no network calls (or what task 7 changed).
- `iarc.md`: the answers for the age-rating questionnaire. There is no user-generated content, no chat and no purchases; share links are created by the user.
- The Store logos come from `npx tauri icon` (`StoreLogo.png` and the `Square*Logo.png` files already exist in `src-tauri/icons`). Regenerate them if the logo changes.

### 10. Submission checklist (human, in Partner Center) — `msstore` keeps it current

1. storedeveloper.microsoft.com → **Company** account (Polytropo, free). Complete the verification of the company and the domain `meetburner.com`.
2. Apps and games → **New product → EXE or MSI app** → reserve "MeetBurner".
3. **Packages**:
   - the URL from task 4;
   - architecture x64;
   - app type **EXE**;
   - silent parameter `/S`;
   - languages en-US and es-ES.

   Run **package validation** and fix every finding before submitting.
4. **Properties**:
   - category Productivity;
   - privacy URL `https://meetburner.com/privacy`;
   - website `https://meetburner.com`;
   - support contact;
   - system requirements: Windows 10 x64, and a webcam for the overlay. Windows 11 is needed for "MeetBurner Camera (Windows 11)".
5. **Age ratings**: fill the IARC questionnaire with `docs/store/iarc.md`.
6. **Store listings** (en, es): paste from `docs/store/listing.*.md`, upload the screenshots and logos.
7. **Pricing and availability**: free, in all markets.
8. **Notes for certification**: paste `docs/store/certification-notes.md`.
9. Submit. If certification fails, copy the report into an issue labelled `store`, and fix the problem in a new version (a new tag and a new URL).

### Every later release

- Tag as usual (ADR-0025). After the release is published, create a **new submission** in Partner Center with the new `_store-setup.exe` URL and the "What's new" text from `docs/store/`.
- Never reuse or overwrite a URL that was already submitted.

## Rules

- Follow `.github/copilot-instructions.md`: work test-first, write docs in English, keep user-facing strings in English and Spanish, and verify the same way CI does.
- Never commit certificates, tokens, `.pfx` files, Azure IDs that grant access, or Partner Center credentials.
  CI reads them from secrets or OIDC only.
- Do not change the camera registration model (HKCU, per-user, opt-in) or the frame pipeline (ADR-0013, ADR-0019).
- Do not move to MSIX, and do not add the Store config to the default build, without a new ADR.
- Keep `docs/user-guide.md`, `docs/code-signing-policy.md` and `README.md` true. When a task changes behaviour (uninstall, updates, signer name), update them in the same PR.
- After each task, tick it in `docs/plan/roadmap.md` (M13).

## References

- Tauri v2, Microsoft Store: https://v2.tauri.app/distribute/microsoft-store/
- Tauri v2, Windows code signing: https://v2.tauri.app/distribute/sign/windows/
- MSI/EXE submission: https://learn.microsoft.com/windows/apps/publish/publish-your-app/msi/create-app-submission
- MSI/EXE certification: https://learn.microsoft.com/windows/uwp/publish/msiexe/the-app-certification-process
- Package validation: https://learn.microsoft.com/windows/apps/publish/publish-your-app/package-validation-pre-check
- Store policies: https://learn.microsoft.com/windows/apps/publish/store-policies
- Code signing options: https://learn.microsoft.com/windows/apps/package-and-deploy/code-signing-options
