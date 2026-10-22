# 0003. Cost model and salary presets

- Status: accepted
- Date: 2026-10-22

## Context

Users rarely know hourly rates. They know (or can estimate) a yearly gross salary.

## Decision

- Each participant has an **annual gross salary**, picked from a list of typical salaries in Spain or typed manually.
- `hourlyRate = annualSalary / hoursPerYear`, `hoursPerYear` defaults to **1800** and is user-configurable.
- Cost is computed from timestamps on each animation frame for a fluid counter (cents visibly ticking).
- Presets live in `packages/core/src/salaries-es.ts` as approximate gross EUR/year values; they are editable defaults, not authoritative data.
- Changing currency only changes formatting; amounts are not converted.

## Open options (not in MVP)

- Employer cost multiplier (Spanish social security ≈ +30%).
- Currency conversion of presets.
- Presets for other countries.
