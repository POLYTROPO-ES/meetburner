import { findCountry, type CountryCode } from './countries';
import { presetSalary } from './salaries';

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
  country: CountryCode;
  currency: string;
  hoursPerYear: number;
  /** When true, cost = gross salary + employer contributions (total cost). */
  includeEmployerCost: boolean;
  /** Fraction of gross salary, e.g. 0.32 for +32%. */
  employerCostRate: number;
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
  return applyCountry(
    {
      country: 'ES',
      currency: 'EUR',
      hoursPerYear: DEFAULT_HOURS_PER_YEAR,
      includeEmployerCost: true,
      employerCostRate: 0,
      participants: [],
      startAt: null,
      endAt: null,
    },
    'ES',
  );
}

/** Sets the country defaults and re-prices participants that use a preset. */
export function applyCountry(meeting: Meeting, code: CountryCode): Meeting {
  const country = findCountry(code);
  return {
    ...meeting,
    country: country.code,
    currency: country.currency,
    hoursPerYear: country.hoursPerYear,
    employerCostRate: country.employerCostRate,
    participants: meeting.participants.map((p) => ({
      ...p,
      annualSalary: presetSalary(p.presetId, country.code) ?? p.annualSalary,
    })),
  };
}

export function hourlyRate(annualSalary: number, hoursPerYear = DEFAULT_HOURS_PER_YEAR, employerCostRate = 0): number {
  if (!(annualSalary > 0) || !(hoursPerYear > 0)) return 0;
  return (annualSalary * (1 + Math.max(0, employerCostRate || 0))) / hoursPerYear;
}

export function participantHourlyRate(p: Participant, meeting: Meeting): number {
  const extra = meeting.includeEmployerCost ? meeting.employerCostRate : 0;
  return hourlyRate(p.annualSalary, meeting.hoursPerYear, extra);
}

export function totalHourlyRate(meeting: Meeting): number {
  return meeting.participants.reduce((sum, p) => sum + participantHourlyRate(p, meeting), 0);
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
