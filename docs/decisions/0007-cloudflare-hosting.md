# 0007. Host web apps on Cloudflare

- Status: accepted
- Date: 2026-10-22

## Decision

- All web apps are deployed to **Cloudflare Pages** as static sites (the web app has no backend).
- Deploy: `npm run deploy:web` (builds, then `wrangler pages deploy apps/web/dist --project-name meetcost`). CI will run the same command with a `CLOUDFLARE_API_TOKEN` secret (M0).
- Wrangler runs through `npx` with a pinned major version instead of being a dev dependency, so installs stay small.
- Security headers live in `apps/web/public/_headers`: strict CSP, `frame-ancestors 'none'`, `nosniff`, `no-referrer` and a Permissions-Policy that turns off camera, microphone and geolocation for the web app.

## Consequences

- If a backend is needed later (e.g. shared meetings), use Cloudflare Workers/D1 so it stays on the same platform, with a new ADR.
