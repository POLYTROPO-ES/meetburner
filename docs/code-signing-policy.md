# Code signing policy

Status: **application pending**. MeetBurner binaries are built by public CI from the tagged
source, but the SignPath Foundation application is still in progress (roadmap M9), so **current
releases are unsigned**. The policy below is what applies once it is approved.

## Who signs the releases

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

Every release is built by GitHub Actions from a tag in this repository
(`.github/workflows/release.yml`), and every signing request is approved by a project approver
before the artifacts are published (ADR-0027).

## Roles

- Authors (committers): [POLYTROPO-ES members](https://github.com/orgs/POLYTROPO-ES/people)
- Reviewers: every change reaches `main` through a pull request and CI
- Approvers: [POLYTROPO-ES owners](https://github.com/orgs/POLYTROPO-ES/people?query=role%3Aowner)

## What is signed

- `MeetBurnerCamera.dll`, the DirectShow virtual camera
- `meetburner.exe` and the NSIS/MSI installers

`SHA256SUMS.txt` is published with every release so a download can be checked independently of
the signature.

## Privacy

This program will not transfer any information to other networked systems unless specifically
requested by the user or the person installing or operating it.

- No telemetry, no analytics, no crash reporting.
- The web app runs entirely in the browser; a share link carries the meeting summary in the URL
  fragment, which is never sent to a server.
- The desktop app talks to no network service.

## System changes

Registering the virtual camera writes a per-user COM registration under `HKCU`: no administrator
rights and no system-wide driver install. *Clean webcam for meets* removes it, and uninstalling
the app removes it too (ADR-0017, ADR-0020).
