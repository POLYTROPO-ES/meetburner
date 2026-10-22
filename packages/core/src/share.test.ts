import { describe, expect, it } from 'vitest';
import { createMeeting, decodeSummary, encodeSummary, meetingSummary, type Meeting } from './index';

const meeting = (overrides: Partial<Meeting> = {}): Meeting => ({
  ...createMeeting(),
  includeEmployerCost: false,
  participants: [{ id: 'a', name: 'Ana', presetId: null, annualSalary: 108000 }], // 60 €/h
  ...overrides,
});

describe('meetingSummary', () => {
  it('is null when the meeting has not started', () => {
    expect(meetingSummary(meeting(), 1_000)).toBeNull();
    expect(meetingSummary(meeting({ startAt: 5_000 }), 1_000)).toBeNull();
  });

  it('takes total time, total cost and cost per minute from the meeting', () => {
    const s = meetingSummary(meeting({ startAt: 0, endAt: 30 * 60_000 }), 99 * 60_000);
    expect(s).toEqual({ elapsedMs: 30 * 60_000, totalCost: 30, costPerMinute: 1, currency: 'EUR' });
  });
});

describe('share link', () => {
  const summary = { elapsedMs: 3_723_000, totalCost: 1234.56, costPerMinute: 19.88, currency: 'USD' };

  it('round-trips a summary', () => {
    expect(decodeSummary(encodeSummary(summary))).toEqual(summary);
  });

  it('only carries the aggregate numbers, never names or salaries', () => {
    expect(encodeSummary(summary)).toBe('t=3723&c=1234.56&m=19.88&cur=USD');
  });

  it('rounds to whole seconds and cents', () => {
    const q = encodeSummary({ elapsedMs: 1_499, totalCost: 0.129, costPerMinute: 0.004, currency: 'EUR' });
    expect(decodeSummary(q)).toEqual({ elapsedMs: 1_000, totalCost: 0.13, costPerMinute: 0, currency: 'EUR' });
  });

  it('rejects missing, negative, non-numeric or tampered values', () => {
    for (const q of [
      '',
      'garbage',
      't=10&c=1&m=1',
      't=-1&c=1&m=1&cur=EUR',
      't=10&c=abc&m=1&cur=EUR',
      't=10&c=1&m=Infinity&cur=EUR',
      't=10&c=1&m=1&cur=<script>',
      't=10&c=1e400&m=1&cur=EUR',
    ]) {
      expect(decodeSummary(q), q).toBeNull();
    }
  });
});
