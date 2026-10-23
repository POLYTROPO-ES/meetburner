# MeetBurner user guide

MeetBurner shows what a meeting costs while it happens: participants, salaries and a running
counter. It runs in a browser at [meetburner.com](https://meetburner.com) and as a desktop app
that can publish the counter as a **virtual camera**, so the cost appears inside Meet, Teams,
Zoom or any other app that reads a camera.

## What each version can do

| Platform | Scope |
|---|---|
| **Windows** desktop | Everything: setup page, counter window, webcam overlay and the *MeetBurner Camera* virtual camera |
| **Linux** desktop | Setup page, counter window and share links; no virtual camera yet |
| **macOS** desktop | Setup page, counter window and share links; no virtual camera yet, and the app is not notarized |
| **Web** | Everything a browser can do: setup, counter window, share links. A browser cannot publish a virtual camera |

## Install

Download from [the latest release](https://github.com/POLYTROPO-ES/meetburner/releases/latest).

**Windows**

- `MeetBurner_<version>_x64-setup.exe` (NSIS) or `..._x64_en-US.msi` — recommended; installs into
  your user profile and adds an entry to *Apps & features*.
- `MeetBurner_<version>_x64.exe` — portable, a single file. The first time you register the
  camera, the driver is written next to the exe, so keep it in a writable folder.
- `MeetBurner_<version>_portable_x64.zip` — the same portable exe plus
  `resources\MeetBurnerCamera.dll` already on disk. Prefer this one if antivirus or SmartScreen
  flags the single-file build: an unsigned exe that drops a DLL and registers a COM server looks
  like a dropper to heuristics.

**macOS**

- `MeetBurner_<version>_universal.dmg` (Apple silicon and Intel).
- The build is **not signed or notarized** yet. Gatekeeper blocks the first launch: right-click
  the app, choose *Open*, then confirm. The camera is not part of the macOS build.

**Linux**

- `..._amd64.deb` or `..._amd64.AppImage` (x86_64).

**Web**

Open [meetburner.com](https://meetburner.com). Nothing to install.

## First run

1. Pick the country and currency (or type a salary directly).
2. Add participants and adjust their gross salary. The salaries are rough presets, not advice;
   the employer-cost switch adds the contributions on top.
3. Press **Start** (or **Open counter**) to show the counter window, **Stop** to freeze it.
4. When the meeting ends, **Share the cost** copies a link that contains the summary.

Everything you type stays in the browser's local storage on that device. There is no account.

## The counter

- **Layout** — *Fullscreen* fills the window; *Widget* draws a compact card, used when the
  counter is overlaid on a webcam.
- **Style** — seven presets (Classic, Modern, Formal, Halloween, Christmas, 4 seasons,
  Minimalist). Every preset remembers its own tweaks:
  - each text element (Title, Cost, Status, Elapsed, Person-time, Cost per minute) can have its
    own font, size and colour;
  - the background can be solid or a gradient between two colours, horizontal or vertical, with
    opacity from 0 (invisible) to 100 (solid) and a corner radius from square to very rounded;
  - **Reset style** returns the selected preset to its original look.
- **Large amounts** — abbreviate (K, M, B) or shrink the font when the cost outgrows the card.
- The *4 seasons* style follows the northern hemisphere; there is no southern option yet.

## Virtual camera (Windows)

1. Open the desktop app and scroll to **Camera**.
2. Press **Register webcam for meets** once per Windows account. It writes a per-user `HKCU`
   registration, so no administrator prompt appears. The app also replaces an older driver with
   the one bundled in the build.
3. In your meeting app choose **MeetBurner Camera** as the camera. The counter is live while
   MeetBurner is open, so you can preview it before joining.
4. **Clean webcam for meets** removes the registration. Uninstalling the app removes it too.

Notes:

- Enumerating cameras loads the driver into that process. Chrome, Edge and some meeting apps
  keep it loaded until they close; that blocks building over it, nothing else.
- Moving or renaming the app folder breaks an existing registration (the DLL path is recorded).
  Register again from the new location.
- Meeting apps keep the frame size they negotiated when they opened the camera. The counter is
  always scaled to fit; reselect the camera only if you want a different frame shape.
- The overlay card can be dragged and resized on the webcam preview. The camera image and the
  card can be mirrored independently.
- The camera can be overlaid on a real webcam or stand alone. If the webcam is busy or
  unplugged, the counter keeps running on its own.

## Language

English and Spanish, in the app and in the counter. The first visit picks the language from the
browser and remembers your choice.

## Privacy

- No telemetry, no analytics, no crash reporting, no account.
- The desktop app talks to no network service. Camera frames travel through local shared memory
  between the app and the driver.
- A share link carries the meeting summary in the URL fragment (`#/share?...`), which browsers do
  not send to a server.
- The web app is served from Cloudflare Pages; that is the only network request it makes.

Details and the code signing policy: [code-signing-policy.md](code-signing-policy.md).

## Troubleshooting

**Windows shows "Windows protected your PC" (SmartScreen).**
The build is not code signed yet. Choose *More info*, then *Run anyway*. The zip build is less
likely to trigger it.

**The camera does not appear in Meet/Teams/Zoom.**
Open the app's **Camera** section and check the status line. If it says *Not registered*, press
**Register webcam for meets**. If it says *Registered*, close the meeting app completely (not
just the window) and reopen the camera list: apps cache the camera list at start.

**The camera disappeared after moving the app.**
The registration points at a file path. Move the app back, or press *Clean webcam for meets* and
register again from the new folder.

**The counter window does not open in the browser.**
It is a popup: allow popups for meetburner.com, or use the *Open counter* link the setup page
offers when the popup is blocked.

**"This share link is not valid."**
The summary is encoded in the link; a truncated URL (some chat apps cut long links) cannot be
recovered. Send it as plain text.

**The macOS app is "damaged" or cannot be opened.**
Gatekeeper: right-click the app, *Open*, confirm. Do not remove the quarantine attribute unless
you trust the download.

**Uninstall.**
Windows: *Apps & features*. Portable builds: delete the folder after pressing *Clean webcam for
meets*. Linux: your package manager, or delete the AppImage. Registration lives in `HKCU`, so a
Windows uninstall removes it with no leftovers.

## Reporting a problem

Open an issue at
[github.com/POLYTROPO-ES/meetburner/issues](https://github.com/POLYTROPO-ES/meetburner/issues)
with the version (bottom of the setup page or the installer name), the platform and what you
expected to happen.
