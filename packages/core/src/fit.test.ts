import { describe, expect, it } from 'vitest';
import { DEFAULT_DISPLAY, fitCost, formatMoneyCompact, normalizeDisplay } from './index';

describe('formatMoneyCompact', () => {
  it('keeps small amounts as normal money', () => {
    expect(formatMoneyCompact(999.5, 'USD', 'en-US')).toBe('$999.50');
  });

  it('abbreviates with K, M, B and T, keeping the currency position', () => {
    expect(formatMoneyCompact(12_345.67, 'USD', 'en-US')).toBe('$12.35K');
    expect(formatMoneyCompact(1_234_567, 'USD', 'en-US')).toBe('$1.23M');
    expect(formatMoneyCompact(2_500_000_000, 'GBP', 'en-US')).toBe('£2.50B');
    expect(formatMoneyCompact(7e12, 'USD', 'en-US')).toBe('$7.00T');
    expect(formatMoneyCompact(12_345.67, 'EUR', 'es-ES')).toBe('12,35K €');
  });

  it('moves to the next unit instead of showing 1,000.00K', () => {
    expect(formatMoneyCompact(999_999, 'USD', 'en-US')).toBe('$1.00M');
  });

  it('uses the long scale in Spanish: 10^9 is "mil M" and B is a European billion (10^12)', () => {
    expect(formatMoneyCompact(1_234_567, 'EUR', 'es-ES')).toBe('1,23M €');
    expect(formatMoneyCompact(2_500_000_000, 'EUR', 'es-ES')).toBe('2,50 mil M €');
    expect(formatMoneyCompact(7e12, 'EUR', 'es-ES')).toBe('7,00B €');
    expect(formatMoneyCompact(2_500_000_000, 'USD', 'es-ES')).toBe('$2,50 mil M');
  });
});

describe('fitCost', () => {
  it('does nothing when the amount fits', () => {
    expect(fitCost(500, 400, 300, 'compact')).toEqual({ compact: false, scale: 1 });
    expect(fitCost(500, 400, 300, 'shrink')).toEqual({ compact: false, scale: 1 });
  });

  it('abbreviates in compact mode when the amount is too wide', () => {
    expect(fitCost(500, 800, 400, 'compact')).toEqual({ compact: true, scale: 1 });
  });

  it('also shrinks in compact mode when even the abbreviation is too wide', () => {
    expect(fitCost(500, 900, 800, 'compact')).toEqual({ compact: true, scale: 0.62 });
  });

  it('shrinks the font in shrink mode when the amount is too wide', () => {
    expect(fitCost(500, 800, 400, 'shrink')).toEqual({ compact: false, scale: 0.62 });
  });

  it('ignores unmeasured (zero) sizes', () => {
    expect(fitCost(0, 0, 0, 'shrink')).toEqual({ compact: false, scale: 1 });
  });
});

describe('overflow display setting', () => {
  it('defaults to compact and validates stored values', () => {
    expect(DEFAULT_DISPLAY.overflow).toBe('compact');
    expect(normalizeDisplay({ overflow: 'shrink' }).overflow).toBe('shrink');
    expect(normalizeDisplay({ overflow: 'explode' }).overflow).toBe('compact');
  });
});
