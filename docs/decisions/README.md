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
