import type { CountryCode } from './countries';

export interface SalaryPreset {
  id: string;
  label: { en: string; es: string };
  /** Rough gross annual salary per country, in the country's currency. */
  annual: Record<CountryCode, number>;
}

type Row = [id: string, en: string, es: string, ...annual: number[]];

// Rough estimates (2025–2026), to be replaced by official sources later.
// Column order must match COUNTRY_ORDER.
const COUNTRY_ORDER: CountryCode[] = ['ES', 'PT', 'FR', 'DE', 'IT', 'NL', 'GB', 'US', 'MX'];
const ROWS: Row[] = [
  // id, en, es,                                       ES,     PT,     FR,     DE,     IT,     NL,     GB,     US,      MX
  ['intern', 'Intern', 'Becario/a',                     12000,   9000,   8000,  14000,   9600,  12000,  18000,  35000,    90000],
  ['minimum-wage', 'Minimum wage', 'Salario mínimo',    16576,  12180,  21622,  26600,  18000,  28000,  23800,  15080,   101000],
  ['admin', 'Administrative assistant', 'Administrativo/a', 22000, 15000, 26000, 38000, 24000, 34000,  25000,  42000,   150000],
  ['customer-support', 'Customer support', 'Atención al cliente', 21000, 13000, 25000, 34000, 22000, 32000, 23000, 40000, 120000],
  ['sales', 'Sales representative', 'Comercial',       30000,  20000,  35000,  50000,  32000,  45000,  32000,  65000,   250000],
  ['marketing', 'Marketing specialist', 'Especialista de marketing', 32000, 22000, 38000, 52000, 33000, 48000, 35000, 70000, 280000],
  ['hr', 'HR specialist', 'Técnico/a de RR. HH.',      32000,  22000,  38000,  52000,  33000,  48000,  35000,  68000,   260000],
  ['accountant', 'Accountant', 'Contable',              30000,  21000,  38000,  50000,  32000,  46000,  35000,  70000,   240000],
  ['designer', 'UX/UI designer', 'Diseñador/a UX/UI',   33000,  22000,  40000,  55000,  32000,  50000,  40000,  90000,   300000],
  ['qa', 'QA engineer', 'Ingeniero/a QA',               30000,  22000,  40000,  55000,  32000,  50000,  40000,  85000,   300000],
  ['dev-junior', 'Junior developer', 'Desarrollador/a junior', 24000, 18000, 38000, 48000, 28000, 42000, 32000, 80000, 240000],
  ['dev-mid', 'Mid-level developer', 'Desarrollador/a mid', 35000, 28000, 48000, 62000, 36000, 58000, 50000, 115000, 420000],
  ['dev-senior', 'Senior developer', 'Desarrollador/a senior', 48000, 40000, 60000, 75000, 48000, 72000, 70000, 150000, 650000],
  ['devops', 'DevOps engineer', 'Ingeniero/a DevOps',  45000,  36000,  55000,  70000,  45000,  68000,  65000, 140000,   600000],
  ['data-scientist', 'Data scientist', 'Científico/a de datos', 45000, 35000, 55000, 70000, 42000, 65000, 60000, 135000, 550000],
  ['tech-lead', 'Tech lead', 'Tech lead',               58000,  50000,  72000,  90000,  58000,  85000,  85000, 175000,   800000],
  ['product-manager', 'Product manager', 'Product manager', 50000, 40000, 65000, 80000, 50000, 75000, 70000, 145000,   650000],
  ['project-manager', 'Project manager', 'Jefe/a de proyecto', 45000, 35000, 55000, 70000, 45000, 65000, 55000, 110000, 450000],
  ['eng-manager', 'Engineering manager', 'Engineering manager', 70000, 60000, 85000, 105000, 70000, 100000, 100000, 200000, 950000],
  ['lawyer', 'Lawyer', 'Abogado/a',                     45000,  30000,  60000,  75000,  45000,  70000,  65000, 150000,   450000],
  ['consultant', 'Consultant', 'Consultor/a',           40000,  30000,  50000,  65000,  40000,  60000,  50000, 100000,   400000],
  ['director', 'Director', 'Director/a',                90000,  70000, 120000, 140000, 100000, 130000, 120000, 220000,  1200000],
  ['ceo', 'CEO', 'CEO',                                120000, 100000, 180000, 200000, 150000, 180000, 180000, 300000,  2000000],
];

export const SALARY_PRESETS: readonly SalaryPreset[] = ROWS.map(([id, en, es, ...values]) => ({
  id,
  label: { en, es },
  annual: Object.fromEntries(COUNTRY_ORDER.map((c, i) => [c, values[i]])) as Record<CountryCode, number>,
}));

export function findPreset(id: string | null): SalaryPreset | undefined {
  return id === null ? undefined : SALARY_PRESETS.find((p) => p.id === id);
}

export function presetSalary(id: string | null, country: CountryCode): number | undefined {
  return findPreset(id)?.annual[country];
}
