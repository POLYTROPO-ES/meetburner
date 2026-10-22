export type CountryCode = 'ES' | 'PT' | 'FR' | 'DE' | 'IT' | 'NL' | 'GB' | 'US' | 'MX';

export interface Country {
  code: CountryCode;
  name: { en: string; es: string };
  currency: string;
  /** Typical worked hours per year. */
  hoursPerYear: number;
  /** Employer social contributions as a fraction of gross salary (rough estimate). */
  employerCostRate: number;
}

export const COUNTRIES: readonly Country[] = [
  { code: 'ES', name: { en: 'Spain', es: 'España' }, currency: 'EUR', hoursPerYear: 1800, employerCostRate: 0.32 },
  { code: 'PT', name: { en: 'Portugal', es: 'Portugal' }, currency: 'EUR', hoursPerYear: 1800, employerCostRate: 0.2375 },
  { code: 'FR', name: { en: 'France', es: 'Francia' }, currency: 'EUR', hoursPerYear: 1607, employerCostRate: 0.45 },
  { code: 'DE', name: { en: 'Germany', es: 'Alemania' }, currency: 'EUR', hoursPerYear: 1700, employerCostRate: 0.21 },
  { code: 'IT', name: { en: 'Italy', es: 'Italia' }, currency: 'EUR', hoursPerYear: 1750, employerCostRate: 0.3 },
  { code: 'NL', name: { en: 'Netherlands', es: 'Países Bajos' }, currency: 'EUR', hoursPerYear: 1700, employerCostRate: 0.2 },
  { code: 'GB', name: { en: 'United Kingdom', es: 'Reino Unido' }, currency: 'GBP', hoursPerYear: 1800, employerCostRate: 0.15 },
  { code: 'US', name: { en: 'United States', es: 'Estados Unidos' }, currency: 'USD', hoursPerYear: 1900, employerCostRate: 0.1 },
  { code: 'MX', name: { en: 'Mexico', es: 'México' }, currency: 'MXN', hoursPerYear: 2200, employerCostRate: 0.25 },
];

export function findCountry(code: string): Country {
  return COUNTRIES.find((c) => c.code === code) ?? COUNTRIES[0];
}
