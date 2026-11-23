# Notes for certification

MeetBurner is a productivity app that shows the running cost of a meeting. It has no account, no
login and makes no network calls.

## What to test

1. Install the app. The Store runs the installer with `/S`; it installs per user and needs no
   administrator prompt.
2. The setup page opens. In the Camera section, press **Register webcam for meets**.
3. That writes a per-user COM registration under `HKCU` for `MeetBurnerCamera.dll` (DirectShow).
   No administrator rights and no kernel driver are involved. On Windows 11 the app can also
   register **MeetBurner Camera (Windows 11)** through `MFCreateVirtualCamera`
   (`MB_MfRegister` in `MeetBurnerMfCamera.dll`).
4. Open the Windows Camera app or https://webcamtests.com in Edge and choose
   **MeetBurner Camera** (or **MeetBurner Camera (Windows 11)** on Windows 11). The counter is
   live while MeetBurner is open.
5. Press **Clean webcam for meets** to remove the registration, or uninstall the app. The
   uninstaller calls `meetburner.exe --unregister-cameras` before removing the files. It cleans the
   Windows account that runs the uninstaller; other accounts can use **Clean webcam for meets**.
6. Uninstalling silently (`/S`) leaves no camera registration for that account.

## Notes

- The app is a Win32 EXE/MSI, not MSIX. The camera registration is per-user `HKCU` and has to be
  visible to Zoom, Teams, Chrome/Edge and the frame server; an MSIX container would virtualise it.
- The webcam is used only locally. Camera frames travel through local shared memory between the app
  and the camera driver. There is no telemetry, analytics or crash reporting.
- The app does not check for updates. Users update from the Store page or the GitHub release page.
- The Store installer includes the WebView2 runtime (offline installer), so it also installs
  without a network connection.
- If a meeting app does not show the camera, close it completely and reopen it: meeting apps cache
  the camera list at start. Moving the app folder breaks the recorded DLL path; register again.
- Test account: none. Support: https://github.com/POLYTROPO-ES/meetburner/issues
