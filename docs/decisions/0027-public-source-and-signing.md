# 0027. Public source under MIT, binaries signed by SignPath Foundation

- Status: accepted (SignPath application pending)
- Date: 2026-10-23

## Context

The source is about to be published. Two things were blocking that:

1. **No license.** Without one the code is "all rights reserved", so nobody may legally reuse it
   and no signing programme will accept the project: SignPath Foundation requires an
   OSI-approved license without commercial dual-licensing.
2. **No viable signing path.** ADR-0025 assumed a base64 PFX in `WINDOWS_CERTIFICATE`. Since
   June 2023 the CA/Browser Forum requires code-signing private keys to live in FIPS 140-2
   hardware, so a newly issued certificate arrives on a USB token or as a cloud service and
   cannot be exported into GitHub secrets. A USB token is unreachable from a hosted runner.

SignPath Foundation exists for exactly this case: it holds the certificate on its HSM, verifies
that each artifact was built by CI from an open repository, and signs for free.

## Decision

- **License:** MIT, copyright POLYTROPO-ES. Shortest OSI-approved license that fits a consumer
  app; no copyleft, so packagers can ship it.
- **Signing:** apply to SignPath Foundation; on approval the release workflow submits signing
  requests instead of importing a PFX. This amends the certificate part of ADR-0025 — the
  tag → tests → draft release flow stays exactly as it is.
- **Policy in the open:** `docs/code-signing-policy.md` carries the required wording, the team
  roles and the privacy statement; it is linked from `README.md` and from the web app footer,
  which is the project's home page for the purpose of the SignPath review.
- **Until approval:** releases ship unsigned and the release notes say so (existing behaviour in
  `release.yml`).
- **Credential:** the only CI secret left is `SIGNPATH_API_TOKEN`, which can submit signing
  requests but cannot sign on its own.

## Consequences

- Every release gains one manual approval step: a tag builds, an approver signs, then the draft
  is published. The "tag is the release" property of ADR-0025 becomes "tag is the build".
- Users see *SignPath Foundation* as the publisher, not MeetBurner.
- Builds must keep working from the public repository only; a private submodule or a private
  registry would break origin verification.
- SignPath can reject the application at its discretion. Fallback: Azure Trusted Signing
  (~10 USD/month, OIDC, still no stored secret).
