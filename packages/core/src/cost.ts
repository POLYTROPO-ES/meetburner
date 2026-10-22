export const DEFAULT_HOURS_PER_YEAR = 1800;
const MS_PER_HOUR = 3_600_000;

export interface Participant {
  id: string;
  name: string;
  /** Salary preset id, or null for a custom salary. */
  presetId: string | null;
  /** Gross annual salary in the meeting currency. */
  annualSalary: number;
}

export interface Meeting {
  currency: string;
  hoursPerYear: number;
  participants: Participant[];
  /** Epoch ms. */
  startAt: number | null;
  /** Epoch ms; set when the meeting is stopped. */
  endAt: number | null;
}

export interface MeetingSnapshot {
  elapsedMs: number;
  totalCost: number;
  personTimeMs: number;
  costPerMinute: number;
  /** > 0 when startAt is in the future. */
  startsInMs: number;
  running: boolean;
}

export const CURRENCIES = ['EUR', 'USD', 'GBP', 'CHF', 'MXN', 'ARS', 'COP', 'CLP', 'BRL'] as const;

export function createMeeting(): Meeting {
  return {
    currency: 'EUR',
    hoursPerYear: DEFAULT_HOURS_PER_YEAR,
    participants: [],
    startAt: null,
    endAt: null,
  };
}

export function hourlyRate(annualSalary: number, hoursPerYear = DEFAULT_HOURS_PER_YEAR): number {
  if (!(annualSalary > 0) || !(hoursPerYear > 0)) return 0;
  return annualSalary / hoursPerYear;
}

export function totalHourlyRate(meeting: Meeting): number {
  return meeting.participants.reduce((sum, p) => sum + hourlyRate(p.annualSalary, meeting.hoursPerYear), 0);
}

export function meetingSnapshot(meeting: Meeting, now: number): MeetingSnapshot {
  const ratePerHour = totalHourlyRate(meeting);
  const costPerMinute = ratePerHour / 60;
  const { startAt, endAt } = meeting;
  if (startAt === null) {
    return { elapsedMs: 0, totalCost: 0, personTimeMs: 0, costPerMinute, startsInMs: 0, running: false };
  }
  const end = endAt === null ? now : Math.min(endAt, now);
  const elapsedMs = Math.max(0, end - startAt);
  return {
    elapsedMs,
    totalCost: (ratePerHour * elapsedMs) / MS_PER_HOUR,
    personTimeMs: elapsedMs * meeting.participants.length,
    costPerMinute,
    startsInMs: Math.max(0, startAt - now),
    running: endAt === null && now >= startAt,
  };
}
