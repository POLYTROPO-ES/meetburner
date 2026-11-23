# Microsoft Store submission checklist

Human steps, in Partner Center. The code side is ready when the signing route is configured and a
release contains `MeetBurner_<version>_x64_store-setup.exe` (ADR-0039).

## Before submitting

- [ ] `SIGNING_ROUTE` is configured (`signpath` or `artifact-signing`) and the release contains a
      signed `MeetBurner_<version>_x64_store-setup.exe`.
- [ ] `https://meetburner.com/privacy` answers 200 in production (the page is deployed on merge,
      ADR-0012).
- [ ] Silent install, silent upgrade and silent uninstall pass on clean Windows 10 and 11 VMs
      (`docs/store/silent-install-checks.md`).
- [ ] The Store package validation in Partner Center passes.
- [ ] The "What's new" text in `docs/store/listing.*.md` matches this release.
- [ ] `docs/store/screenshots/` has the four current images (see its README).
- [ ] The Store logos in `apps/desktop/src-tauri/icons/` are current (`StoreLogo.png` and the
      `Square*Logo.png` files).

## Partner Center

1. storedeveloper.microsoft.com → **Company** account (Polytropo). Complete the verification of
   the company and the domain `meetburner.com`.
2. Apps and games → **New product → EXE or MSI app** → reserve **MeetBurner**.
3. **Packages**
   - URL:
     `https://github.com/POLYTROPO-ES/meetburner/releases/download/vX.Y.Z/MeetBurner_X.Y.Z_x64_store-setup.exe`
   - Architecture: x64.
   - App type: EXE.
   - Silent parameter: `/S`.
   - Languages: en-US and es-ES.
   - Run **package validation** and fix every finding before submitting.
4. **Properties**
   - Category: Productivity.
   - Privacy URL: `https://meetburner.com/privacy`.
   - Website: `https://meetburner.com`.
   - Support contact: `https://github.com/POLYTROPO-ES/meetburner/issues`.
   - System requirements: Windows 10 x64; a webcam for the overlay; Windows 11 for
     "MeetBurner Camera (Windows 11)".
5. **Age ratings**: fill the IARC questionnaire with `docs/store/iarc.md`.
6. **Store listings** (en, es): paste from `docs/store/listing.en.md` and
   `docs/store/listing.es.md`, and upload the screenshots and logos.
7. **Pricing and availability**: free, all markets.
8. **Notes for certification**: paste `docs/store/certification-notes.md`.
9. Submit. If certification fails, copy the report into an issue labelled `store` and fix the
   problem in a new version (a new tag and a new URL).

## Every later release

- Tag as usual (ADR-0025). After the release is published, create a **new submission** in Partner
  Center with the new `_store-setup.exe` URL and the "What's new" text from `docs/store/`.
- Never reuse or overwrite a URL that was already submitted. A broken Store build means a new
  version and a new tag (ADR-0039).
