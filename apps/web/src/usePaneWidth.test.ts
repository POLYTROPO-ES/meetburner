import { describe, expect, it } from 'vitest';
import { clampPane, PANE_DEFAULT, PANE_MAX, PANE_MIN, parsePane } from './usePaneWidth';

describe('the width of the editor column', () => {
  it('keeps a dragged width between the limits', () => {
    expect(clampPane(0.1)).toBe(PANE_MIN);
    expect(clampPane(0.9)).toBe(PANE_MAX);
    expect(clampPane(0.33)).toBe(0.33);
  });

  it('falls back to the default for junk', () => {
    expect(clampPane(Number.NaN)).toBe(PANE_DEFAULT);
    expect(parsePane(null)).toBe(PANE_DEFAULT);
    expect(parsePane('')).toBe(PANE_DEFAULT);
    expect(parsePane('nope')).toBe(PANE_DEFAULT);
  });

  it('reads a stored fraction', () => {
    expect(parsePane('0.4')).toBe(0.4);
    // Out-of-range stored values are clamped, not trusted.
    expect(parsePane('2')).toBe(PANE_MAX);
  });
});
