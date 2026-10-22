import { describe, expect, it } from 'vitest';
import {
  createMeeting,
  detectLang,
  formatDuration,
  formatMoney,
  fromLocalInputValue,
  hourlyRate,
  meetingSnapshot,
  MESSAGES,
  toLocalInputValue,
  totalHourlyRate,
  type Meeting,
} from './index';

const meeting = (overrides: Partial<Meeting> = {}): Meeting => ({
  ...createMeeting(),
  includeEmployerCost: false,
  participants: [
    { id: 'a', name: 'A', presetId: null, annualSalary: 36000 },
    { id: 'b', name: 'B', presetId: null, annualSalary: 18000 },
  ],
  ...overrides,
});

describe('hourlyRate', () => {
  it('divides annual salary by hours per year (default 1800)', () => {
    expect(hourlyRate(36000)).toBe(20);
    expect(hourlyRate(36000, 1200)).toBe(30);
  });
  it('returns 0 for invalid input', () => {
    expect(hourlyRate(0)).toBe(0);
    expect(hourlyRate(-1)).toBe(0);
    expect(hourlyRate(36000, 0)).toBe(0);
    expect(hourlyRate(Number.NaN)).toBe(0);
  });
});

describe('meetingSnapshot', () => {
  it('sums rates of all participants', () => {
    expect(totalHourlyRate(meeting())).toBe(30);
  });

  it('is idle when not started', () => {
    const s = meetingSnapshot(meeting(), 1000);
    expect(s).toMatchObject({ elapsedMs: 0, totalCost: 0, running: false, costPerMinute: 0.5 });
  });

  it('computes cost and person-time from elapsed time', () => {
    const s = meetingSnapshot(meeting({ startAt: 0 }), 3_600_000);
    expect(s.elapsedMs).toBe(3_600_000);
    expect(s.totalCost).toBeCloseTo(30);
    expect(s.personTimeMs).toBe(7_200_000);
    expect(s.running).toBe(true);
  });

  it('freezes at endAt', () => {
    const s = meetingSnapshot(meeting({ startAt: 0, endAt: 1_800_000 }), 9_999_999);
    expect(s.totalCost).toBeCloseTo(15);
    expect(s.running).toBe(false);
  });

  it('reports startsIn for a future start', () => {
    const s = meetingSnapshot(meeting({ startAt: 10_000 }), 4_000);
    expect(s).toMatchObject({ elapsedMs: 0, totalCost: 0, startsInMs: 6_000, running: false });
  });
});

describe('format', () => {
  it('formats durations as HH:MM:SS', () => {
    expect(formatDuration(0)).toBe('00:00:00');
    expect(formatDuration(3_723_999)).toBe('01:02:03');
    expect(formatDuration(-5)).toBe('00:00:00');
  });
  it('formats money with two decimals', () => {
    expect(formatMoney(1234.5, 'USD', 'en-US')).toBe('$1,234.50');
  });
  it('falls back on invalid currency', () => {
    expect(formatMoney(1, 'NOPE!', 'en')).toBe('1.00 NOPE!');
  });
  it('round-trips datetime-local values', () => {
    const ms = new Date(2026, 9, 22, 9, 30).getTime();
    expect(fromLocalInputValue(toLocalInputValue(ms))).toBe(ms);
    expect(fromLocalInputValue('')).toBeNull();
    expect(fromLocalInputValue('garbage')).toBeNull();
  });
});

describe('i18n', () => {
  it('has the same keys in every language', () => {
    expect(Object.keys(MESSAGES.es).sort()).toEqual(Object.keys(MESSAGES.en).sort());
  });
  it('brands the app as MeetBurner with its slogan in every language', () => {
    expect(MESSAGES.en.appTitle).toBe('MeetBurner');
    expect(MESSAGES.es.appTitle).toBe('MeetBurner');
    expect(MESSAGES.en.tagline).toBe('Every meeting burns money.');
    expect(MESSAGES.en.headline).toBe('See the cost. Feel the burn.');
    expect(MESSAGES.en.description).toBe("A live meeting-cost overlay that shows the combined cost of everyone's time.");
    for (const key of ['tagline', 'headline', 'description'] as const) expect(MESSAGES.es[key]).not.toBe(MESSAGES.en[key]);
  });
  it('detects Spanish locales', () => {
    expect(detectLang('es-ES')).toBe('es');
    expect(detectLang('en-GB')).toBe('en');
    expect(detectLang(undefined)).toBe('en');
  });
});
