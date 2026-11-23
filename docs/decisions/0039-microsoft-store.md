# 0039. Microsoft Store: EXE/MSI submission, a separate store build and a clean uninstall

- Status: accepted
- Date: 2026-11-23

## Context

The Windows desktop app ships as NSIS and MSI installers from GitHub Releases (ADR-0025).
The Microsoft Store accepts Win32 EXE/MSI products, but its policies add requirements the
current releases do not meet:

- the installer and the binaries behind it must be code signed with a certificate that chains
  to the Microsoft Trusted Root Program (policy 10.2.9);
- the installer must install silently (`/S` for NSIS);
- uninstalling must remove what the app installed (policy 10.2.7);
- the download URL registered in Partner Center must keep returning the same binary.

The camera model is the part that constrains the packaging choice. **Register webcam for meets**
writes a per-user COM registration under `HKCU` and records the DLL path in `InprocServer32`
(ADR-0016, ADR-0017); on Windows 11 the MF camera is registered through `MFCreateVirtualCamera`
(ADR-0031). Meeting apps and the frame server read that registration from the user's hive.

Since June 2023 a code-signing private key must live in a FIPS 140-2 HSM and cannot be
exported to a PFX secret. ADR-0027 already replaced the PFX plan with SignPath Foundation;
Azure Artifact Signing (formerly Trusted Signing) is the fallback. Neither credential exists
yet, so the workflow must keep building unsigned for forks and dry runs.

## Decision

1. **Submit as an EXE/MSI product, never MSIX.** An MSIX container virtualises writes to
   `HKCU\Software\Classes` into a private per-package hive, so Zoom, Teams, Chrome and the frame
   server would not see the DirectShow registration or the MF camera. The install folder under
   `WindowsApps` also changes with every version, which breaks the recorded `InprocServer32`
   path. MSIX would need packaged COM and restricted capabilities; that is a new decision, not a
   packaging detail.
2. **The Store build is a separate Tauri configuration.** `tauri.microsoftstore.conf.json`
   selects the offline WebView2 installer (about +127 MB) and sets the publisher to
   `Polytropo`. The GitHub installers keep `tauri.conf.json` and the smaller bootstrapper.
   The product name cannot equal the publisher, and without the explicit value Tauri derives
   `meetburner` from the identifier, which the Store rejects.
3. **No PFX.** `release.yml` supports two routes, selected by the repository variable
   `SIGNING_ROUTE`; anything else (or missing credentials) produces the existing unsigned build:
   - `signpath` (primary, ADR-0027): two signing requests through
     `signpath/github-action-submit-signing-request@v3`. The DirectShow and MF DLLs are signed
     first because `build.rs` embeds the DirectShow DLL in the exe (ADR-0026); the exe and the
     NSIS/MSI installers are signed in a second request after `tauri build`.
   - `artifact-signing` (fallback): `artifact-signing-cli` (the tool documented by Tauri) signs
     the DLLs before the build, then Tauri's `bundle.windows.signCommand` signs the exe and the
     installers. The CLI authenticates with an Azure App Registration (`AZURE_CLIENT_ID`,
     `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`); it has no OIDC mode, so this route stores one
     client secret.
   - Every signed run verifies the DLLs, the exe and both installers with
     `signtool verify /pa /v` and fails when an expected signature is missing.
4. **The Store asset is immutable.** The release publishes
   `MeetBurner_<version>_x64_store-setup.exe` next to the normal assets. The publish job refuses
   to run against a release that is already published, refuses to overwrite an existing
   `*_store-setup.exe`, and does not pass `--clobber` for it. A bad Store build means a new
   version and a new tag, never a re-upload.
5. **The camera model does not change.** It stays per-user, under `HKCU`, with no administrator
   rights and no kernel driver, and registration remains an explicit user action (ADR-0017).
6. **Uninstalling removes the cameras.** `meetburner.exe --unregister-cameras` runs the
   DirectShow unregistration and `MB_MfUnregister` (when the MF camera is registered), then exits
   with code 0 without opening a window. The NSIS `NSIS_HOOK_PREUNINSTALL` hook and a WiX custom
   action call it. **Limitation:** the uninstaller cleans the Windows account that runs it;
   other accounts keep their per-user keys until they press *Clean webcam for meets* or open a
   newer version.
7. **No in-app update check.** The Store does not update EXE/MSI apps, so users update from the
   Store page or the GitHub release. MeetBurner keeps the property that the desktop app talks to
   no network service, and the privacy statement does not change.
8. **Privacy page.** `https://meetburner.com/privacy` publishes the privacy section in English
   and Spanish and is linked from the web footer. It is the URL the Store listing points to.

## Consequences

- Until one signing route is active, the Store cannot accept the upload. The workflow and the
  ADR are ready; enabling a route is setting `SIGNING_ROUTE` plus its credentials.
- SignPath adds a manual approval step per build (ADR-0027). Artifact Signing costs about
  10 USD/month and keeps the publisher name `Polytropo`, but stores one App Registration secret.
- The Store installer is about 127 MB larger than the GitHub installer because WebView2 is
  embedded. Both installers are built from the same commit and version.
- The Store asset URL is a permanent reference. Re-running a release workflow after publication
  is refused by design; a fix always ships as a new version.
- Users must update manually. The listing and the user guide say so.
- Other Windows accounts are not cleaned by an uninstall from a different account. The user guide
  and the certification notes describe the *Clean webcam for meets* fallback.
