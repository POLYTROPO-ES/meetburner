# 0011. Short vs long scale for abbreviated amounts

- Status: accepted
- Date: 2026-10-22
- Amends: [0010](0010-large-amounts.md) (the "abbreviate" suffixes)

## Context

"Billion" means 10^9 in the US and UK (short scale). In Spain and most of continental Europe, a billion (billón) is 10^12, and 10^9 is "mil millones" (long scale). Showing `$2.50B` to a Spanish reader would overstate the cost.

## Decision

The abbreviation follows the UI locale, because the reader is the one who interprets it:

| Locale | 10^3 | 10^6 | 10^9 | 10^12 |
|--------|------|------|------|-------|
| `en-*` (short scale) | K | M | B | T |
| others, e.g. `es-ES` (long scale) | K | M | mil M | B (billón) |

## Consequences

- `2,50 mil M €` in Spanish and `€2.50B` in English mean the same amount.
- New UI languages use the long scale unless they start with `en`.
