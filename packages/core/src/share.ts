import { meetingSnapshot, type Meeting } from './cost';

/** Aggregate meeting stats that can be shared publicly (no names or salaries). */
export interface MeetingSummary {
  elapsedMs: number;
  totalCost: number;
  costPerMinute: number;
  currency: string;
}

export function meetingSummary(meeting: Meeting, now: number): MeetingSummary | null {
  if (meeting.startAt === null || meeting.startAt > now) return null;
  const { elapsedMs, totalCost, costPerMinute } = meetingSnapshot(meeting, now);
  return { elapsedMs, totalCost, costPerMinute, currency: meeting.currency };
}

const cents = (v: number) => Math.round(v * 100) / 100;

/** Encodes a summary as a short query string for the `#/share?...` link. */
export function encodeSummary(s: MeetingSummary): string {
  return new URLSearchParams({
    t: String(Math.floor(s.elapsedMs / 1000)),
    c: String(cents(s.totalCost)),
    m: String(cents(s.costPerMinute)),
    cur: s.currency,
  }).toString();
}

/** Parses a share query string; returns null if anything is missing or invalid. */
export function decodeSummary(query: string): MeetingSummary | null {
  const params = new URLSearchParams(query);
  const num = (key: string) => {
    const raw = params.get(key);
    const value = raw === null || raw.trim() === '' ? NaN : Number(raw);
    return Number.isFinite(value) && value >= 0 ? value : null;
  };
  const seconds = num('t');
  const totalCost = num('c');
  const costPerMinute = num('m');
  const currency = params.get('cur') ?? '';
  if (seconds === null || totalCost === null || costPerMinute === null || !/^[A-Z]{3}$/.test(currency)) {
    return null;
  }
  return { elapsedMs: Math.floor(seconds) * 1000, totalCost: cents(totalCost), costPerMinute: cents(costPerMinute), currency };
}
