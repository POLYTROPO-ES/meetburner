# 0009. Share links carry the summary in the URL

- Status: accepted
- Date: 2026-10-22

## Context

Users want a "Share the cost" link that shows a meeting summary (total time, total cost, cost per minute). The web app is static on Cloudflare Pages (ADR-0007) with no backend or database.

## Decision

- The summary is encoded in the link itself: `#/share?t=<seconds>&c=<total>&m=<per minute>&cur=<ISO currency>`. Plain `URLSearchParams`, no base64 and no dependency.
- Only aggregate numbers are shared. Names, salaries and roles never leave the device.
- `decodeSummary` validates the data: numbers must be finite and non-negative, and the currency must be three uppercase letters. Anything else shows "invalid link".
- The desktop app (Tauri) has no public origin, so it builds links against `VITE_PUBLIC_URL` (default `https://meetburner.pages.dev/`).

## Consequences

- There is nothing to store, no privacy risk and no server cost.
- Links can be edited by hand, so a shared figure is not proof of anything. That is acceptable for a fun or awareness tool.
- Values are rounded to whole seconds and cents.
