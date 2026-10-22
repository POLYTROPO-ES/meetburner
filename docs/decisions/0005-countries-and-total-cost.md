# 0005. Countries and total (employer) cost

- Status: accepted
- Date: 2026-10-22
- Supersedes: [0003](0003-cost-model-and-salaries.md)

## Context

Users want the real cost of a meeting for the company, not only gross salaries, and they're not all in Spain.

## Decision

- The meeting has a **country** (ES, PT, FR, DE, IT, NL, GB, US, MX to start). Choosing a country sets its defaults:
  currency, working hours per year and employer contribution rate. Users can still change each value.
- Salary presets are one shared role list with a **rough gross annual salary per country** in local currency
  (`packages/core/src/salaries.ts`). When the country changes, participants who use a preset are re-priced;
  custom salaries stay as they are.
- **Total cost is on by default**:
  `hourlyRate = annualSalary × (1 + employerCostRate) / hoursPerYear`. Users can turn it off to see gross only.
- Spain keeps the default of 1800 h/year.
- Every value is a rough estimate for 2025–2026. A later milestone will switch to official sources (INE, Eurostat,
  ONS, BLS, INEGI…) and record the source and date next to each value.

## Consequences

- Adding a country = one entry in `countries.ts` + one column in `salaries.ts`. Tests check that every preset has a value for every country.
- Amounts aren't converted between currencies. Changing only the currency is for formatting.
