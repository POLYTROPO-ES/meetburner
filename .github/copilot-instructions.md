# MeetBurner – repository instructions

- Read [mvp.md](../mvp.md), [docs/architecture.md](../docs/architecture.md) and [docs/plan/roadmap.md](../docs/plan/roadmap.md) before starting work.
- Code, comments and docs in English. User-facing strings in English and Spanish via `packages/core/src/i18n.ts`.
- Keep it simple: no new dependencies or abstractions unless a milestone needs them.
- Work test-first (TDD, ADR-0008): red (failing test) -> green (minimal code) -> refactor. Never add logic without a test.
- Business logic belongs in `packages/core` (pure TS, no dependencies) and must have Vitest tests.
- Any non-trivial technical choice gets an ADR in `docs/decisions/`.
- After finishing a roadmap item, tick it in `docs/plan/roadmap.md`.
- Verify before finishing: `npm test`, `npm run build:web`; for Rust: `cargo fmt --check && cargo clippy && cargo test` in `apps/desktop/src-tauri`.
- Web apps are deployed to Cloudflare Pages (ADR-0007). Commits are small and descriptive (one concern per commit, body explains why).
