# Silent install, upgrade and uninstall checks

The Store runs the installer unattended. Run these checks on a **clean** Windows 10 x64 VM and a
**clean** Windows 11 x64 VM before submitting, with the Store installer from the release:

```text
https://github.com/POLYTROPO-ES/meetburner/releases/download/vX.Y.Z/MeetBurner_X.Y.Z_x64_store-setup.exe
```

Record the results in the release PR (or in the submission issue). The Store runs exactly the
`/S` install below.

## 1. Silent install without a network connection

1. Disconnect the VM from the network (the Store installer embeds WebView2).
2. Run, from a normal PowerShell:

   ```powershell
   .\MeetBurner_X.Y.Z_x64_store-setup.exe /S
   $LASTEXITCODE
   ```

3. Expected: exit code `0`, no UI, `%LOCALAPPDATA%\MeetBurner\meetburner.exe` exists, and the app
   opens from the Start menu.
4. Open the app → Camera → **Register webcam for meets**. Then check:

   ```powershell
   reg query "HKCU\Software\Classes\CLSID\{DBD3A890-1830-4222-AFEE-B1BA2C2DAB49}"
   reg query "HKCU\Software\Classes\CLSID\{9D6A2B41-3C57-4A0E-8F2D-7B1E5C3A9F64}"
   ```

   The first key must exist on Windows 10 and 11. The second only on Windows 11 (the MF camera).
5. Reconnect the network for the remaining checks.

## 2. Silent upgrade

1. Keep the current settings (country, participants, style) and the camera registered.
2. Install a newer `MeetBurner_X.Y.(Z+1)_x64_store-setup.exe` with `/S`.
3. Expected: exit code `0`, the app version is the new one, the settings are intact, and the
   camera registration still exists. The installer runs the previous uninstaller with `/UPDATE`
   and must not remove the HKCU keys (ADR-0039; NSIS `$UpdateMode`, MSI `UPGRADINGPRODUCTCODE`).

## 3. Silent uninstall

1. Run the uninstaller silently. For the NSIS per-user install:

   ```powershell
   & "$env:LOCALAPPDATA\MeetBurner\uninstall.exe" /S
   $LASTEXITCODE
   ```

2. Expected: exit code `0`, the install folder is gone, and both registry queries from step 1
   return "not found". Open the Windows Camera app or https://webcamtests.com in Edge: the
   MeetBurner cameras are gone.
3. The uninstaller cleans the Windows account that runs it. Sign in with a second account that
   registered the camera: its keys remain until it presses **Clean webcam for meets** (ADR-0039).

## Result table

| Check | Windows 10 VM | Windows 11 VM |
|---|---|---|
| `/S` install, exit 0, no UI, offline | pending | pending |
| App opens, both cameras register | pending | pending |
| `/S` upgrade keeps settings and registration | pending | pending |
| `/S` uninstall removes both registrations | pending | pending |

Replace `pending` with the observed result (and the installer version) in the PR that ships the
release.
