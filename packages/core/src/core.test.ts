import { describe, expect, it } from 'vitest';
import {
  createMeeting,
  detectLang,
  findPreset,
  formatDuration,
  formatMoney,
  fromLocalInputValue,
  hourlyRate,
  meetingSnapshot,
  MESSAGES,
  SALARY_PRESETS_ES,
  toLocalInputValue,
  totalHourlyRate,
  type Meeting,
} from './index';

const meeting = (overrides: Partial<Meeting> = {}): Meeting => ({
  ...createMeeting(),
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
  it('detects Spanish locales', () => {
    expect(detectLang('es-ES')).toBe('es');
    expect(detectLang('en-GB')).toBe('en');
    expect(detectLang(undefined)).toBe('en');
  });
});

describe('salary presets', () => {
  it('have unique ids and positive salaries', () => {
    const ids = SALARY_PRESETS_ES.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(SALARY_PRESETS_ES.every((p) => p.annual > 0)).toBe(true);
  });
  it('can be looked up by id', () => {
    expect(findPreset('dev-senior')?.annual).toBeGreaterThan(0);
    expect(findPreset(null)).toBeUndefined();
  });
});
