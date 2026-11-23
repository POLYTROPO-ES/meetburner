# Code signing policy

Status: **application pending**. MeetBurner binaries are built by public CI from the tagged
source, but the SignPath Foundation application is still in progress (roadmap M9) and no other
signing credential is configured yet, so **current releases are unsigned**. The policy below is
what applies once a route is enabled.

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
- `MeetBurnerMfCamera.dll`, the Windows 11 Media Foundation virtual camera
- `meetburner.exe` and the NSIS/MSI installers

`SHA256SUMS.txt` is published with every release so a download can be checked independently of
the signature.

## Release configuration

`release.yml` does not import a PFX: since June 2023 a code-signing private key lives in an HSM
and cannot be exported to a GitHub secret. The route is the repository variable
`SIGNING_ROUTE`:

| Value | Signing | Credentials |
|---|---|---|
| `signpath` (primary, ADR-0027) | SignPath Foundation | secret `SIGNPATH_API_TOKEN`; variables `SIGNPATH_ORGANIZATION_ID`, `SIGNPATH_PROJECT_SLUG`, `SIGNPATH_SIGNING_POLICY_SLUG`, `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG` |
| `artifact-signing` (fallback, ADR-0039) | Azure Artifact Signing | secrets `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`; variables `AZURE_ARTIFACT_SIGNING_ENDPOINT`, `AZURE_ARTIFACT_SIGNING_ACCOUNT`, `AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE` |
| unset or incomplete | none: unsigned build (forks and dry runs) | none |

The order is fixed: the camera DLLs are signed **before** `tauri build`, because `build.rs`
embeds the DirectShow DLL in the exe (ADR-0026); `meetburner.exe` is signed next; the NSIS and
MSI installers are built from the signed exe and signed last. Every signed run verifies the two
DLLs, the exe and both installers with `signtool verify /pa /v`; a missing signature fails the
job.

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
