# 0012. Deploy on push with GitHub Actions, served at meetburner.com

- Status: accepted
- Date: 2026-10-22
- Extends: [0007](0007-cloudflare-hosting.md)

## Context

ADR-0007 deploys with `npm run deploy:web`, so the same command should run in CI. Cloudflare's Git integration can only be set up in the dashboard, and it would build with its own image and settings. Wrangler 4 also sends `pages` commands to Workers by default.

## Decision

- The Pages project `meetburner` is a classic Pages project created with `wrangler pages project create --force`. It uses direct upload, and its production branch is `main`.
- `.github/workflows/deploy-web.yml` runs `npm ci`, `npm test` and `npm run deploy:web` on every push to `main`. It can also be started by hand.
- Secrets: `CLOUDFLARE_API_TOKEN`, a token limited to *Cloudflare Pages: Edit*, and `CLOUDFLARE_ACCOUNT_ID`.
- Custom domains are `meetburner.com` and `www.meetburner.com`, both attached to the Pages project. Share links from the desktop app default to `https://meetburner.com/`.

## Consequences

- Failing tests block the deploy.
- The build and deploy are the same locally and in CI.
- The project is not linked to Git in the Cloudflare dashboard, so pushes do not trigger a second build there.
