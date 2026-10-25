import { describe, expect, it } from 'vitest';
import {
  clampOverlay,
  COUNTER_ELEMENTS,
  DEFAULT_DISPLAY,
  defaultOverlayRect,
  FLIP_MS,
  FLIP_SHADE,
  flapFrame,
  flapProgress,
  FONTS,
  isPlated,
  MESSAGES,
  moveOverlay,
  NO_MIRROR,
  normalizeDisplay,
  OVERLAY_MIN,
  PLATED_ELEMENTS,
  PLATE_GAP_EM,
  PLATE_SPACE_EM,
  PLATE_WIDTH_EM,
  platedWidth,
  resizeOverlay,
  resolveFont,
  resolveTheme,
  seasonOf,
  styleDefaults,
  THEME_IDS,
  THEMES,
} from './index';

describe('themes', () => {
  it('offer the eight selectable styles', () => {
    expect(THEME_IDS).toEqual([
      'classic',
      'modern',
      'formal',
      'halloween',
      'christmas',
      'seasons',
      'minimalist',
      'splitflap',
    ]);
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

/** WCAG 2.2 relative luminance of a #rrggbb colour. */
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5]
    .map((start) => parseInt(hex.slice(start, start + 2), 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG 2.2 contrast ratio between two #rrggbb colours. */
function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

describe('theme contrast (WCAG AA)', () => {
  // The card paints a gradient between the two backgrounds, so both must pass.
  const backgrounds = (theme: (typeof THEMES)[keyof typeof THEMES]) => [theme.background, theme.backgroundAlt];

  it.each(Object.entries(THEMES))('%s keeps text readable on both backgrounds', (_id, theme) => {
    for (const background of backgrounds(theme)) {
      expect(contrast(theme.text, background)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(Object.entries(THEMES))('%s keeps muted text readable on both backgrounds', (_id, theme) => {
    for (const background of backgrounds(theme)) {
      expect(contrast(theme.muted, background)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it.each(Object.entries(THEMES))('%s keeps the accent readable for the large cost figure', (_id, theme) => {
    // The cost is large text (>= 24 px or >= 18.66 px bold), so 3:1 is the AA threshold.
    for (const background of backgrounds(theme)) {
      expect(contrast(theme.accent, background)).toBeGreaterThanOrEqual(3);
    }
  });
});

describe('split-flap plates (ADR-0036)', () => {
  it('declare plates on every preset, with splitflap as the only flap style', () => {
    for (const theme of Object.values(THEMES)) expect(['none', 'flap']).toContain(theme.plates);
    const date = new Date(2026, 9, 25);
    expect(THEME_IDS.filter((id) => resolveTheme(id, date).plates === 'flap')).toEqual(['splitflap']);
  });

  it('resolve splitflap to the airport-board preset', () => {
    expect(resolveTheme('splitflap', new Date(2026, 9, 25))).toEqual({
      background: '#0b0c0f',
      backgroundAlt: '#1e2126',
      text: '#f7f3e8',
      muted: '#a7a294',
      accent: '#ffb020',
      font: 'condensed',
      weight: 700,
      radius: 4,
      decoration: '',
      plates: 'flap',
    });
  });

  it('put a solid background behind the board and keep the gradient elsewhere', () => {
    expect(styleDefaults(THEMES.splitflap).background.mode).toBe('solid');
    expect(styleDefaults(THEMES.classic).background.mode).toBe('gradient');
  });

  it('plate the value elements and leave the captions plain', () => {
    expect(PLATED_ELEMENTS).toEqual(['cost', 'elapsed', 'personTime', 'costPerMinute']);
    for (const element of PLATED_ELEMENTS) expect(isPlated(THEMES.splitflap, element)).toBe(true);
    for (const element of ['label', 'status'] as const) expect(isPlated(THEMES.splitflap, element)).toBe(false);
    for (const element of COUNTER_ELEMENTS) expect(isPlated(THEMES.classic, element)).toBe(false);
  });

  it('measure a plated row exactly as wide as it looks', () => {
    const plate = (PLATE_WIDTH_EM + PLATE_GAP_EM) * 100;
    const gap = PLATE_GAP_EM * 100;
    expect(plate).toBe(80);
    expect(PLATE_SPACE_EM * 100).toBe(40);
    expect(platedWidth('1234', 100)).toBe(4 * plate - gap);
    expect(platedWidth('12:34', 100)).toBe(5 * plate - gap);
    expect(platedWidth('1 2', 100)).toBe(plate + 40 + plate - gap);
    expect(platedWidth('', 100)).toBe(0);
    expect(platedWidth('  ', 100)).toBe(80);
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
  it('default to classic, compact and no stored styles', () => {
    expect(DEFAULT_DISPLAY).toEqual({
      theme: 'classic',
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
    const d = normalizeDisplay({ theme: 'halloween', overflow: 'shrink' });
    expect(d).toMatchObject({ theme: 'halloween', overflow: 'shrink' });
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

describe('split-flap flip (ADR-0037)', () => {
  const settled = (glyph: 'from' | 'to') => ({ glyph, scale: 1, shade: 0 });

  it('starts on the old glyph with both halves settled', () => {
    expect(flapFrame(0)).toEqual({ top: settled('from'), bottom: settled('from') });
  });

  it('drops the old glyph in the top half during the first half of the flip', () => {
    const frame = flapFrame(0.25);
    expect(frame.top.glyph).toBe('from');
    expect(frame.top.scale).toBeCloseTo(Math.SQRT1_2, 4);
    expect(frame.top.shade).toBeCloseTo(FLIP_SHADE * (1 - Math.SQRT1_2), 4);
    expect(frame.bottom).toEqual(settled('from'));
  });

  it('shows the new glyph settled once the flap passes the seam', () => {
    expect(flapFrame(0.5)).toEqual({ top: settled('to'), bottom: settled('from') });
  });

  it('drops the old glyph in the bottom half during the second half of the flip', () => {
    const frame = flapFrame(0.75);
    expect(frame.top).toEqual(settled('to'));
    expect(frame.bottom.glyph).toBe('from');
    expect(frame.bottom.scale).toBeCloseTo(Math.SQRT1_2, 4);
    expect(frame.bottom.shade).toBeCloseTo(FLIP_SHADE * (1 - Math.SQRT1_2), 4);
  });

  it('ends with both halves on the new glyph', () => {
    expect(flapFrame(1)).toEqual({ top: settled('to'), bottom: settled('to') });
  });

  it('clamps out-of-range progress to the ends of the flip', () => {
    expect(flapFrame(-1)).toEqual(flapFrame(0));
    expect(flapFrame(2)).toEqual(flapFrame(1));
  });

  it('keeps every shade within 0..FLIP_SHADE', () => {
    for (let i = -5; i <= 105; i++) {
      const frame = flapFrame(i / 100);
      for (const half of [frame.top, frame.bottom]) {
        expect(half.shade).toBeGreaterThanOrEqual(0);
        expect(half.shade).toBeLessThanOrEqual(FLIP_SHADE);
      }
    }
  });

  it('turns elapsed time into 0..1 progress of one FLIP_MS fall', () => {
    expect(flapProgress(1000, 1000 + FLIP_MS / 2)).toBe(0.5);
    expect(flapProgress(1000, 900)).toBe(0);
    expect(flapProgress(1000, 5000)).toBe(1);
  });
});
