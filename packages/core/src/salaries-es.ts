export interface SalaryPreset {
  id: string;
  /** Approximate gross annual salary in Spain, EUR. */
  annual: number;
  label: { en: string; es: string };
}

// Approximate market references; users can override any value.
export const SALARY_PRESETS_ES: readonly SalaryPreset[] = [
  { id: 'intern', annual: 12000, label: { en: 'Intern', es: 'Becario/a' } },
  { id: 'minimum-wage', annual: 16576, label: { en: 'Minimum wage (SMI)', es: 'Salario mínimo (SMI)' } },
  { id: 'admin', annual: 22000, label: { en: 'Administrative assistant', es: 'Administrativo/a' } },
  { id: 'customer-support', annual: 21000, label: { en: 'Customer support', es: 'Atención al cliente' } },
  { id: 'sales', annual: 30000, label: { en: 'Sales representative', es: 'Comercial' } },
  { id: 'marketing', annual: 32000, label: { en: 'Marketing specialist', es: 'Especialista de marketing' } },
  { id: 'hr', annual: 32000, label: { en: 'HR specialist', es: 'Técnico/a de RR. HH.' } },
  { id: 'accountant', annual: 30000, label: { en: 'Accountant', es: 'Contable' } },
  { id: 'designer', annual: 33000, label: { en: 'UX/UI designer', es: 'Diseñador/a UX/UI' } },
  { id: 'qa', annual: 30000, label: { en: 'QA engineer', es: 'Ingeniero/a QA' } },
  { id: 'dev-junior', annual: 24000, label: { en: 'Junior developer', es: 'Desarrollador/a junior' } },
  { id: 'dev-mid', annual: 35000, label: { en: 'Mid-level developer', es: 'Desarrollador/a mid' } },
  { id: 'dev-senior', annual: 48000, label: { en: 'Senior developer', es: 'Desarrollador/a senior' } },
  { id: 'devops', annual: 45000, label: { en: 'DevOps engineer', es: 'Ingeniero/a DevOps' } },
  { id: 'data-scientist', annual: 45000, label: { en: 'Data scientist', es: 'Científico/a de datos' } },
  { id: 'tech-lead', annual: 58000, label: { en: 'Tech lead', es: 'Tech lead' } },
  { id: 'product-manager', annual: 50000, label: { en: 'Product manager', es: 'Product manager' } },
  { id: 'project-manager', annual: 45000, label: { en: 'Project manager', es: 'Jefe/a de proyecto' } },
  { id: 'eng-manager', annual: 70000, label: { en: 'Engineering manager', es: 'Engineering manager' } },
  { id: 'lawyer', annual: 45000, label: { en: 'Lawyer', es: 'Abogado/a' } },
  { id: 'consultant', annual: 40000, label: { en: 'Consultant', es: 'Consultor/a' } },
  { id: 'director', annual: 90000, label: { en: 'Director', es: 'Director/a' } },
  { id: 'ceo', annual: 120000, label: { en: 'CEO', es: 'CEO' } },
];

export function findPreset(id: string | null): SalaryPreset | undefined {
  return id === null ? undefined : SALARY_PRESETS_ES.find((p) => p.id === id);
}
