# Architecture Decision Records

One file per decision: `NNNN-short-title.md`. Never rewrite an accepted ADR; supersede it with a new one.

Template:

```md
# NNNN. Title
- Status: proposed | accepted | superseded by NNNN
- Date: YYYY-MM-DD

## Context
## Decision
## Consequences
```

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-tech-stack.md) | Tech stack | accepted |
| [0002](0002-virtual-camera.md) | Virtual camera per OS | accepted |
| [0003](0003-cost-model-and-salaries.md) | Cost model and salary presets | superseded by 0005 |
| [0004](0004-i18n.md) | Languages | accepted |
| [0005](0005-countries-and-total-cost.md) | Countries and total (employer) cost | accepted |
| [0006](0006-counter-styles-and-layouts.md) | Counter styles, layouts and user sizing | accepted |
| [0007](0007-cloudflare-hosting.md) | Host web apps on Cloudflare Pages | accepted |
| [0008](0008-tdd.md) | Test-driven development | accepted |
| [0009](0009-share-links.md) | Share links carry the summary in the URL | accepted |
| [0010](0010-large-amounts.md) | Large amounts: abbreviate or shrink | accepted (suffixes amended by 0011) |
| [0011](0011-short-and-long-scale.md) | Short vs long scale for abbreviated amounts | accepted |
| [0012](0012-deploy-on-push.md) | Deploy on push with GitHub Actions, served at meetburner.com | accepted |
| [0013](0013-windows-frame-ipc.md) | Windows camera frame IPC (shared memory + seqlock) | accepted |
| [0014](0014-renderer-fonts-and-theme-data.md) | Rust renderer: system fonts and generated theme data | accepted |
| [0015](0015-camera-size-and-live-updates.md) | Camera output size, width-filling cost and live updates | accepted |
| [0016](0016-dshow-filter-and-per-user-registration.md) | DirectShow filter: hand-rolled COM and per-user registration | accepted |
| [0017](0017-app-registers-the-driver.md) | The app registers the camera driver | accepted |
| [0018](0018-always-on-camera.md) | The virtual camera is live while the app is open | accepted |
| [0019](0019-camera-frame-orientation-and-mapping.md) | Camera frames: bottom-up rows, one mapping and no flicker | accepted |
| [0020](0020-driver-version-and-self-update.md) | The app updates an outdated driver on start | accepted |
| [0021](0021-webcam-overlay.md) | Webcam overlay: composite the counter over a real camera | accepted |
| [0022](0022-mirroring.md) | Mirroring: the webcam and the counter are flipped independently | accepted |
| [0023](0023-overlay-card-geometry.md) | The counter card can be moved and resized on the camera frame | accepted |
| [0024](0024-adopt-abandoned-mapping.md) | Adopt a camera mapping whose producer is gone | accepted |
| [0025](0025-github-releases.md) | Binaries ship as GitHub Releases built from tags | accepted (code signing amended by 0027) |
| [0026](0026-portable-single-file.md) | The portable build ships as one file and as a folder | accepted |
| [0027](0027-public-source-and-signing.md) | Public source under MIT, binaries signed by SignPath Foundation | accepted (application pending) |
| [0028](0028-macos-without-virtual-camera.md) | macOS ships without the virtual camera until the Apple account exists | accepted |
| [0029](0029-custom-counter-style.md) | Every style is customisable, and the customisation follows the style | accepted |
| [0030](0030-linux-camera-v4l2loopback.md) | Linux camera: use v4l2loopback, never load it | accepted |
| [0031](0031-windows-mf-virtual-camera.md) | Windows Media Foundation virtual camera alongside the DirectShow filter | accepted |
| [0032](0032-one-counter-surface.md) | One place to configure the counter; Layout is derived from the card | accepted |
