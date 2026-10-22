import { describe, expect, it } from 'vitest';
import {
  applyCountry,
  COUNTRIES,
  createMeeting,
  findCountry,
  hourlyRate,
  meetingSnapshot,
  participantHourlyRate,
  presetSalary,
  SALARY_PRESETS,
  type Meeting,
} from './index';

describe('countries', () => {
  it('include Spain, USA, France, UK and Germany at least', () => {
    const codes = COUNTRIES.map((c) => c.code);
    expect(codes).toEqual(expect.arrayContaining(['ES', 'US', 'FR', 'GB', 'DE']));
  });

  it('have a currency, positive hours and a sane employer cost rate', () => {
    for (const c of COUNTRIES) {
      expect(c.currency).toMatch(/^[A-Z]{3}$/);
      expect(c.hoursPerYear).toBeGreaterThan(0);
      expect(c.employerCostRate).toBeGreaterThanOrEqual(0);
      expect(c.employerCostRate).toBeLessThan(1);
    }
  });

  it('default Spain to EUR and 1800 h/year', () => {
    expect(findCountry('ES')).toMatchObject({ currency: 'EUR', hoursPerYear: 1800 });
  });

  it('fall back to Spain for unknown codes', () => {
    expect(findCountry('XX').code).toBe('ES');
  });
});

describe('salary presets', () => {
  it('have unique ids and a positive salary for every country', () => {
    const ids = SALARY_PRESETS.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const p of SALARY_PRESETS) {
      for (const c of COUNTRIES) expect(p.annual[c.code]).toBeGreaterThan(0);
    }
  });

  it('are looked up by id and country', () => {
    expect(presetSalary('dev-senior', 'US')).toBeGreaterThan(presetSalary('dev-senior', 'ES')!);
    expect(presetSalary('unknown', 'ES')).toBeUndefined();
    expect(presetSalary(null, 'ES')).toBeUndefined();
  });
});

describe('employer cost', () => {
  it('adds the employer cost rate on top of the gross salary', () => {
    expect(hourlyRate(36000, 1800, 0.3)).toBeCloseTo(26);
    expect(hourlyRate(36000, 1800, 0)).toBe(20);
  });

  it('applies only when includeEmployerCost is on', () => {
    const m: Meeting = {
      ...createMeeting(),
      employerCostRate: 0.5,
      participants: [{ id: 'a', name: 'A', presetId: null, annualSalary: 36000 }],
    };
    expect(participantHourlyRate(m.participants[0], { ...m, includeEmployerCost: true })).toBe(30);
    expect(participantHourlyRate(m.participants[0], { ...m, includeEmployerCost: false })).toBe(20);
    const snap = meetingSnapshot({ ...m, includeEmployerCost: true, startAt: 0 }, 3_600_000);
    expect(snap.totalCost).toBeCloseTo(30);
  });

  it('is included by default (total cost)', () => {
    expect(createMeeting().includeEmployerCost).toBe(true);
  });
});

describe('applyCountry', () => {
  const base: Meeting = {
    ...createMeeting(),
    participants: [
      { id: 'a', name: 'A', presetId: 'dev-mid', annualSalary: presetSalary('dev-mid', 'ES')! },
      { id: 'b', name: 'B', presetId: null, annualSalary: 12345 },
    ],
  };

  it('switches currency, hours and employer cost to the country defaults', () => {
    const us = findCountry('US');
    expect(applyCountry(base, 'US')).toMatchObject({
      country: 'US',
      currency: us.currency,
      hoursPerYear: us.hoursPerYear,
      employerCostRate: us.employerCostRate,
    });
  });

  it('re-prices preset participants and keeps custom salaries', () => {
    const next = applyCountry(base, 'US');
    expect(next.participants[0].annualSalary).toBe(presetSalary('dev-mid', 'US'));
    expect(next.participants[1].annualSalary).toBe(12345);
  });
});
