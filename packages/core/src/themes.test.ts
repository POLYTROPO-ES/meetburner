import { describe, expect, it } from 'vitest';
import {
  clampOverlay,
  DEFAULT_DISPLAY,
  defaultOverlayRect,
  FONTS,
  MESSAGES,
  moveOverlay,
  NO_MIRROR,
  normalizeDisplay,
  OVERLAY_MIN,
  resizeOverlay,
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
  it('default to classic, fullscreen and no stored styles', () => {
    expect(DEFAULT_DISPLAY).toEqual({
      theme: 'classic',
      layout: 'fullscreen',
      overflow: 'compact',
      mirror: { sourceH: false, sourceV: false, overlayH: false, overlayV: false },
      overlay: null,
      styles: {},
    });
  });

  it('keeps a custom overlay card and rejects broken ones', () => {
    const d = normalizeDisplay({ overlay: { x: 0.1, y: 0.2, w: 0.3, h: 0.4 } });
    expect(d.overlay).toEqual({ x: 0.1, y: 0.2, w: 0.3, h: 0.4 });
    expect(normalizeDisplay({ overlay: { x: 0.9, y: 0.9, w: 0.5, h: 0.5 } }).overlay).toEqual({
      x: 0.5,
      y: 0.5,
      w: 0.5,
      h: 0.5,
    });
    expect(normalizeDisplay({ overlay: { x: 'a' } }).overlay).toBeNull();
    expect(normalizeDisplay({ overlay: { x: 0.1, y: 0.1, w: Number.NaN, h: 0.2 } }).overlay).toBeNull();
    expect(normalizeDisplay({}).overlay).toBeNull();
  });

  it('keeps the mirror flags per layer and rejects non-booleans', () => {
    const d = normalizeDisplay({ mirror: { sourceH: true, overlayV: true, overlayH: 'yes' } });
    expect(d.mirror).toEqual({ sourceH: true, sourceV: false, overlayH: false, overlayV: true });
    expect(normalizeDisplay({ mirror: null }).mirror).toEqual(NO_MIRROR);
  });

  it('normalize unknown or corrupt input to defaults', () => {
    expect(normalizeDisplay(null)).toEqual(DEFAULT_DISPLAY);
    expect(normalizeDisplay('x')).toEqual(DEFAULT_DISPLAY);
    expect(normalizeDisplay({ theme: 'nope', layout: 'tiny', overflow: 'explode' })).toEqual(DEFAULT_DISPLAY);
  });

  it('keep valid values and reject unknown ones', () => {
    const d = normalizeDisplay({ theme: 'halloween', layout: 'widget', overflow: 'shrink' });
    expect(d).toMatchObject({ theme: 'halloween', layout: 'widget', overflow: 'shrink' });
    expect(normalizeDisplay({ overflow: 'explode' }).overflow).toBe('compact');
  });

  it('resolve "theme" font to the theme font stack', () => {
    expect(resolveFont('theme', THEMES.formal)).toBe(FONTS[THEMES.formal.font].stack);
    expect(resolveFont('mono', THEMES.formal)).toBe(FONTS.mono.stack);
  });
});

describe('overlay card geometry', () => {
  it('starts where the renderer puts the widget by default', () => {
    const rect = defaultOverlayRect(1280, 720);
    expect(rect.w).toBeCloseTo(0.42);
    expect(rect.h).toBeCloseTo(0.3);
    expect(rect.x + rect.w).toBeCloseTo(1 - 32 / 1280);
    expect(rect.y + rect.h).toBeCloseTo(1 - 32 / 720);
  });

  it('keeps the card inside the frame and above the minimum size', () => {
    expect(clampOverlay({ x: -1, y: -1, w: 0.01, h: 2 })).toEqual({
      x: 0,
      y: 0,
      w: OVERLAY_MIN,
      h: 1,
    });
    const pushed = clampOverlay({ x: 0.9, y: 0.9, w: 0.5, h: 0.5 });
    expect(pushed.x + pushed.w).toBeLessThanOrEqual(1.0001);
    expect(pushed.y + pushed.h).toBeLessThanOrEqual(1.0001);
  });

  it('drags the card to the edge without shrinking it', () => {
    const rect = { x: 0.3, y: 0.3, w: 0.4, h: 0.3 };
    const dragged = moveOverlay(rect, 0.1, -0.1);
    expect(dragged.x).toBeCloseTo(0.4);
    expect(dragged.y).toBeCloseTo(0.2);
    expect(dragged.w).toBeCloseTo(0.4);
    expect(dragged.h).toBeCloseTo(0.3);

    const right = moveOverlay(rect, 9, 9);
    expect(right.x).toBeCloseTo(0.6, 6);
    expect(right.y).toBeCloseTo(0.7, 6);
    expect(right.w).toBeCloseTo(0.4, 6);

    const left = moveOverlay(rect, -9, -9);
    expect(left.x).toBe(0);
    expect(left.y).toBe(0);
  });

  it('resizes from the corner and stops at the frame edge', () => {
    const rect = { x: 0.5, y: 0.5, w: 0.2, h: 0.2 };
    const bigger = resizeOverlay(rect, 0.1, 0.1);
    expect(bigger.w).toBeCloseTo(0.3);
    expect(bigger.h).toBeCloseTo(0.3);

    const pinned = resizeOverlay(rect, 0.9, 0.9);
    expect(pinned.w).toBeCloseTo(0.5);
    expect(pinned.h).toBeCloseTo(0.5);
    expect(pinned.x + pinned.w).toBeLessThanOrEqual(1.0001);

    const tiny = resizeOverlay(rect, -0.9, -0.9);
    expect(tiny.w).toBe(OVERLAY_MIN);
    expect(tiny.h).toBe(OVERLAY_MIN);
  });
});
