export type Lang = 'en' | 'es';
export const LANGS: readonly Lang[] = ['en', 'es'];

const en = {
  appTitle: 'MeetCost',
  tagline: 'How much is this meeting costing?',
  currency: 'Currency',
  hoursPerYear: 'Working hours per year',
  startTime: 'Meeting start',
  now: 'Now',
  participants: 'Participants',
  name: 'Name',
  role: 'Role',
  customRole: 'Custom salary',
  annualSalary: 'Annual gross salary',
  perHour: '/h',
  addParticipant: 'Add participant',
  remove: 'Remove',
  start: 'Start',
  stop: 'Stop',
  reset: 'Reset',
  openCounter: 'Open counter',
  totalCost: 'Total cost',
  elapsed: 'Elapsed',
  personTime: 'Person-time',
  costPerMinute: 'Cost per minute',
  startsIn: 'Starts in',
  notStarted: 'Not started',
  stopped: 'Stopped',
  noParticipants: 'Add at least one participant.',
  popupBlocked: 'Popup blocked. Allow popups or open the counter here:',
  language: 'Language',
  personSingular: 'person',
  personPlural: 'people',
} as const;

export type MessageKey = keyof typeof en;

const es: Record<MessageKey, string> = {
  appTitle: 'MeetCost',
  tagline: '¿Cuánto está costando esta reunión?',
  currency: 'Moneda',
  hoursPerYear: 'Horas de trabajo al año',
  startTime: 'Inicio de la reunión',
  now: 'Ahora',
  participants: 'Participantes',
  name: 'Nombre',
  role: 'Puesto',
  customRole: 'Salario personalizado',
  annualSalary: 'Salario bruto anual',
  perHour: '/h',
  addParticipant: 'Añadir participante',
  remove: 'Quitar',
  start: 'Iniciar',
  stop: 'Detener',
  reset: 'Reiniciar',
  openCounter: 'Abrir contador',
  totalCost: 'Coste total',
  elapsed: 'Tiempo transcurrido',
  personTime: 'Tiempo-persona',
  costPerMinute: 'Coste por minuto',
  startsIn: 'Empieza en',
  notStarted: 'Sin empezar',
  stopped: 'Detenida',
  noParticipants: 'Añade al menos un participante.',
  popupBlocked: 'Ventana emergente bloqueada. Permítela o abre el contador aquí:',
  language: 'Idioma',
  personSingular: 'persona',
  personPlural: 'personas',
};

export const MESSAGES: Record<Lang, Record<MessageKey, string>> = { en, es };

export function t(lang: Lang, key: MessageKey): string {
  return MESSAGES[lang][key];
}

export function detectLang(locale: string | undefined): Lang {
  return locale?.toLowerCase().startsWith('es') ? 'es' : 'en';
}
