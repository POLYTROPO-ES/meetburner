# Microsoft Store assets

Everything Partner Center needs, except the signing credentials and the submission itself.

| File | Purpose |
|---|---|
| `listing.en.md` | English listing fields (name, short description, description, what's new, features, search terms) and the listing properties. |
| `listing.es.md` | The same in Spanish. |
| `certification-notes.md` | Pasted into **Notes for certification**. Explains the HKCU camera registration, how to test it and what the uninstaller removes. |
| `iarc.md` | Answers for the age-rating questionnaire. |
| `screenshots/` | The Store screenshots and their capture notes. |
| `submission-checklist.md` | The human checklist for Partner Center and every later release. |
| `silent-install-checks.md`, `silent-install-checks.ps1` | The unattended install / upgrade / uninstall checks for clean Windows 10 and 11 VMs. |
| `overlay-schematic.ps1` | Generates the clearly labelled overlay schematic (Windows, System.Drawing). |

The Store logos are in `apps/desktop/src-tauri/icons/`: `StoreLogo.png` and the `Square*Logo.png`
files. They come from `npx tauri icon`; regenerate them if the logo changes.

The Store build is `tauri.microsoftstore.conf.json` (offline WebView2 + `Polytropo` publisher). It
is produced by `.github/workflows/release.yml`; do not upload the GitHub NSIS installer instead.

Current status (2026-11-23): ADR-0039 and M13 tasks 1–5 and 7–9 are implemented in the repository.
Task 2 needs the signing credentials, task 4 is exercised by the first tagged release, task 6 is
the manual VM pass and task 10 is the Partner Center submission.
