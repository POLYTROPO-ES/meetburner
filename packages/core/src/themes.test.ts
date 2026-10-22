import { describe, expect, it } from 'vitest';
import {
  DEFAULT_DISPLAY,
  FONTS,
  MAX_SCALE,
  MIN_SCALE,
  MESSAGES,
  normalizeDisplay,
  resolveFont,
  resolveTheme,
  seasonOf,
  THEME_IDS,
  THEMES,
} from './index';

describe('themes', () => {
  it('offer the seven selectable styles', () => {
    expect(THEME_IDS).toEqual(['classic', 'modern', 'formal', 'halloween', 'christmas', 'seasons', 'minimalist']);
  });

  it('have a translated name for every style', () => {
    for (const id of THEME_IDS) {
      expect(MESSAGES.en[`theme.${id}`]).toBeTruthy();
      expect(MESSAGES.es[`theme.${id}`]).toBeTruthy();
    }
  });

  it('use hex colours so the Rust renderer can parse them', () => {
    for (const theme of Object.values(THEMES)) {
      for (const color of [theme.background, theme.backgroundAlt, theme.text, theme.muted, theme.accent]) {
        expect(color).toMatch(/^#[0-9a-f]{6}$/i);
      }
      expect(FONTS[theme.font]).toBeDefined();
    }
  });
});

describe('seasons', () => {
  it.each([
    [2, 'spring'],
    [5, 'summer'],
    [8, 'autumn'],
    [11, 'winter'],
    [0, 'winter'],
  ] as const)('month %i is %s (northern hemisphere)', (month, season) => {
    expect(seasonOf(new Date(2026, month, 15))).toBe(season);
  });

  it('resolve the "seasons" style to the current season palette', () => {
    expect(resolveTheme('seasons', new Date(2026, 9, 22))).toBe(THEMES.autumn);
    expect(resolveTheme('classic', new Date(2026, 9, 22))).toBe(THEMES.classic);
  });
});

describe('display settings', () => {
  it('default to classic, fullscreen, theme fonts and 100% size', () => {
    expect(DEFAULT_DISPLAY).toEqual({
      theme: 'classic',
      layout: 'fullscreen',
      costFont: 'theme',
      timeFont: 'theme',
      costScale: 1,
      timeScale: 1,
    });
  });

  it('normalize unknown or corrupt input to defaults', () => {
    expect(normalizeDisplay(null)).toEqual(DEFAULT_DISPLAY);
    expect(normalizeDisplay('x')).toEqual(DEFAULT_DISPLAY);
    expect(normalizeDisplay({ theme: 'nope', layout: 'tiny', costFont: 'comic' })).toEqual(DEFAULT_DISPLAY);
  });

  it('keep valid values and clamp sizes', () => {
    const d = normalizeDisplay({ theme: 'halloween', layout: 'widget', costFont: 'mono', costScale: 9, timeScale: 0.1 });
    expect(d).toMatchObject({ theme: 'halloween', layout: 'widget', costFont: 'mono', costScale: MAX_SCALE, timeScale: MIN_SCALE });
  });

  it('resolve "theme" font to the theme font stack', () => {
    expect(resolveFont('theme', THEMES.formal)).toBe(FONTS[THEMES.formal.font].stack);
    expect(resolveFont('mono', THEMES.formal)).toBe(FONTS.mono.stack);
  });
});
