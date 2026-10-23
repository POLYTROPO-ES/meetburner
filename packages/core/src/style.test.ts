import { describe, expect, it } from 'vitest';
import {
  BACKGROUND_MODES,
  COUNTER_ELEMENTS,
  DEFAULT_CAMERA,
  DEFAULT_DISPLAY,
  GRADIENT_DIRECTIONS,
  MAX_OPACITY,
  MAX_RADIUS,
  MAX_SCALE,
  MESSAGES,
  MIN_OPACITY,
  MIN_RADIUS,
  MIN_SCALE,
  normalizeDisplay,
  resetStyle,
  styleDefaults,
  styleFor,
  THEMES,
  cameraInput,
  type DisplaySettings,
  type ElementStyle,
  type MessageKey,
} from './index';
import { createMeeting } from './cost';

const INHERIT: ElementStyle = { font: 'theme', scale: 100, color: null };

const inheritElements = () => ({
  label: { ...INHERIT },
  cost: { ...INHERIT },
  status: { ...INHERIT },
  elapsed: { ...INHERIT },
  personTime: { ...INHERIT },
  costPerMinute: { ...INHERIT },
});

function meeting() {
  return {
    ...createMeeting(),
    startAt: 1_700_000_000_000,
    participants: [
      { id: 'a', name: 'A', presetId: null, annualSalary: 36_000 },
      { id: 'b', name: 'B', presetId: null, annualSalary: 72_000 },
    ],
  };
}

describe('custom counter style', () => {
  it('exposes the elements it can tweak and the limits it accepts', () => {
    expect(COUNTER_ELEMENTS).toEqual(['label', 'cost', 'status', 'elapsed', 'personTime', 'costPerMinute']);
    expect(BACKGROUND_MODES).toEqual(['solid', 'gradient']);
    expect(GRADIENT_DIRECTIONS).toEqual(['horizontal', 'vertical']);
    expect({ MIN_RADIUS, MAX_RADIUS, MIN_OPACITY, MAX_OPACITY }).toEqual({
      MIN_RADIUS: 0,
      MAX_RADIUS: 40,
      MIN_OPACITY: 0,
      MAX_OPACITY: 100,
    });
  });

  it('has a translated label for every control, in both languages', () => {
    const keys: [MessageKey, string, string][] = [
      ['styleElements', 'Text elements', 'Textos'],
      ['element.label', 'Title', 'Título'],
      ['element.cost', 'Cost', 'Coste'],
      ['element.status', 'Status', 'Estado'],
      ['element.elapsed', 'Elapsed', 'Tiempo transcurrido'],
      ['element.personTime', 'Person-time', 'Tiempo-persona'],
      ['element.costPerMinute', 'Cost per minute', 'Coste por minuto'],
      ['styleFont', 'Font', 'Fuente'],
      ['styleSize', 'Size', 'Tamaño'],
      ['styleColor', 'Colour', 'Color'],
      ['styleBackground', 'Background', 'Fondo'],
      ['bg.mode.solid', 'Solid', 'Sólido'],
      ['bg.mode.gradient', 'Gradient', 'Degradado'],
      ['bg.color', 'Colour', 'Color'],
      ['bg.colorAlt', 'Second colour', 'Segundo color'],
      ['bg.direction', 'Direction', 'Dirección'],
      ['bg.horizontal', 'Horizontal', 'Horizontal'],
      ['bg.vertical', 'Vertical', 'Vertical'],
      ['bg.opacity', 'Opacity', 'Opacidad'],
      ['bg.radius', 'Corners', 'Esquinas'],
      ['styleReset', 'Reset style', 'Restablecer estilo'],
    ];
    for (const [key, en, es] of keys) {
      expect(MESSAGES.en[key]).toBe(en);
      expect(MESSAGES.es[key]).toBe(es);
    }
    expect(MESSAGES.en['font.theme']).toBe('Style default');
    expect(MESSAGES.es['font.theme']).toBe('La del estilo');
  });

  it('no longer translates the old, ambiguous "time" element', () => {
    expect('element.time' in MESSAGES.en).toBe(false);
    expect('element.time' in MESSAGES.es).toBe(false);
  });

  it('keeps the decoration only where it reads as one (ADR-0029)', () => {
    expect(THEMES.halloween.decoration).toBe('🎃🦇');
    for (const id of ['christmas', 'spring', 'summer', 'autumn', 'winter'] as const) {
      expect(THEMES[id].decoration).toBe('');
    }
  });
});

describe('styleDefaults', () => {
  it('inherits every element and takes the background from the preset', () => {
    for (const theme of Object.values(THEMES)) {
      const style = styleDefaults(theme);
      expect(style.elements).toEqual(inheritElements());
      for (const element of COUNTER_ELEMENTS) {
        expect(style.elements[element]).toEqual(INHERIT);
      }
      expect(style.background).toEqual({
        mode: 'gradient',
        color: theme.background,
        colorAlt: theme.backgroundAlt,
        direction: 'vertical',
        opacity: 100,
        radius: theme.radius,
      });
    }
  });

  it('gives every preset its own radius', () => {
    expect(styleDefaults(THEMES.classic).background.radius).toBe(12);
    expect(styleDefaults(THEMES.minimalist).background.radius).toBe(0);
    expect(styleDefaults(THEMES.formal).background.radius).toBe(4);
  });
});

describe('styleFor', () => {
  it('returns the preset when the selected style has no stored tweaks', () => {
    expect(styleFor(DEFAULT_DISPLAY, THEMES.classic)).toEqual(styleDefaults(THEMES.classic));
  });

  it('does not leak the tweaks stored for another style', () => {
    const display = normalizeDisplay({
      theme: 'classic',
      styles: { modern: { elements: { cost: { scale: 150 } } } },
    });
    expect(styleFor(display, THEMES.classic)).toEqual(styleDefaults(THEMES.classic));
  });

  it('merges a partial override, keeping the rest of the preset', () => {
    const display = normalizeDisplay({
      theme: 'modern',
      styles: {
        modern: {
          elements: { cost: { font: 'mono', scale: 150 } },
          background: { mode: 'solid', color: '#123456' },
        },
      },
    });
    const style = styleFor(display, THEMES.modern);
    expect(style.elements.cost).toEqual({ font: 'mono', scale: 150, color: null });
    expect(style.elements.label).toEqual(INHERIT);
    expect(style.elements.elapsed).toEqual(INHERIT);
    expect(style.elements.personTime).toEqual(INHERIT);
    expect(style.elements.costPerMinute).toEqual(INHERIT);
    expect(style.elements.status).toEqual(INHERIT);
    expect(style.background).toEqual({
      mode: 'solid',
      color: '#123456',
      colorAlt: THEMES.modern.backgroundAlt,
      direction: 'vertical',
      opacity: 100,
      radius: THEMES.modern.radius,
    });
  });

  it('clamps values that bypassed normalisation', () => {
    const display: DisplaySettings = {
      ...DEFAULT_DISPLAY,
      theme: 'classic',
      styles: {
        classic: {
          elements: {
            label: { ...INHERIT },
            cost: { font: 'mono', scale: 900, color: '#abc' },
            status: { ...INHERIT },
            elapsed: { font: 'theme', scale: 0.01, color: 'not-a-colour' },
            personTime: { ...INHERIT },
            costPerMinute: { ...INHERIT },
          },
          background: {
            mode: 'solid',
            color: '#123456',
            colorAlt: 'oops',
            direction: 'horizontal',
            opacity: 500,
            radius: 999,
          },
        },
      },
    };
    const style = styleFor(display, THEMES.classic);
    expect(style.elements.cost).toEqual({ font: 'mono', scale: MAX_SCALE * 100, color: '#abc' });
    expect(style.elements.elapsed).toEqual({ font: 'theme', scale: MIN_SCALE * 100, color: null });
    expect(style.background).toEqual({
      mode: 'solid',
      color: '#123456',
      colorAlt: THEMES.classic.backgroundAlt,
      direction: 'horizontal',
      opacity: MAX_OPACITY,
      radius: MAX_RADIUS,
    });
  });

  it('takes the background of the season in force for the "seasons" style', () => {
    const display: DisplaySettings = { ...DEFAULT_DISPLAY, theme: 'seasons' };
    expect(styleFor(display, THEMES.autumn).background.color).toBe(THEMES.autumn.background);
  });
});

describe('normalizeDisplay styles', () => {
  it('defaults to no stored styles', () => {
    expect(DEFAULT_DISPLAY.styles).toEqual({});
    expect(normalizeDisplay(null).styles).toEqual({});
    expect(normalizeDisplay({}).styles).toEqual({});
  });

  it('keeps the stored tweaks of each style', () => {
    const d = normalizeDisplay({
      styles: {
        classic: {
          elements: { cost: { font: 'mono', scale: 150, color: '#abc' } },
          background: {
            mode: 'solid',
            color: '#112233',
            colorAlt: '#445566',
            direction: 'horizontal',
            opacity: 40,
            radius: 8,
          },
        },
      },
    });
    expect(d.styles.classic).toEqual({
      elements: { cost: { font: 'mono', scale: 150, color: '#abc' } },
      background: {
        mode: 'solid',
        color: '#112233',
        colorAlt: '#445566',
        direction: 'horizontal',
        opacity: 40,
        radius: 8,
      },
    });
  });

  it('rejects colours that are not #rgb or #rrggbb', () => {
    for (const color of ['red', '#12', '#1234', '#12345', 'rgb(1,2,3)', 42]) {
      expect(normalizeDisplay({ styles: { classic: { elements: { cost: { color } } } } }).styles).toEqual({});
    }
    expect(normalizeDisplay({ styles: { classic: { elements: { cost: { color: '#abc' } } } } }).styles).toEqual({
      classic: { elements: { cost: { color: '#abc' } } },
    });
    expect(normalizeDisplay({ styles: { classic: { elements: { cost: { color: '#AABBCC' } } } } }).styles).toEqual({
      classic: { elements: { cost: { color: '#AABBCC' } } },
    });
  });

  it('rejects a bad background colour with the whole background', () => {
    expect(normalizeDisplay({ styles: { classic: { background: { mode: 'solid', color: 'red' } } } }).styles).toEqual({});
    expect(normalizeDisplay({ styles: { classic: { background: { colorAlt: '#12345' } } } }).styles).toEqual({});
  });

  it('rejects an unknown font', () => {
    expect(normalizeDisplay({ styles: { classic: { elements: { cost: { font: 'comic', scale: 120 } } } } }).styles).toEqual({
      classic: { elements: { cost: { scale: 120 } } },
    });
    expect(normalizeDisplay({ styles: { classic: { elements: { cost: { font: 'mono' } } } } }).styles).toEqual({
      classic: { elements: { cost: { font: 'mono' } } },
    });
  });

  it('rejects an unknown background mode and direction', () => {
    expect(
      normalizeDisplay({ styles: { classic: { background: { mode: 'noise', direction: 'diagonal' } } } }).styles,
    ).toEqual({});
    expect(
      normalizeDisplay({ styles: { classic: { background: { mode: 'solid', direction: 'diagonal' } } } }).styles,
    ).toEqual({ classic: { background: { mode: 'solid' } } });
  });

  it('clamps the sizes and drops values that are not numbers', () => {
    const scaleOf = (scale: unknown) =>
      normalizeDisplay({ styles: { classic: { elements: { cost: { scale } } } } }).styles;
    expect(scaleOf(999)).toEqual({ classic: { elements: { cost: { scale: MAX_SCALE * 100 } } } });
    expect(scaleOf(1)).toEqual({ classic: { elements: { cost: { scale: MIN_SCALE * 100 } } } });
    expect(scaleOf('big')).toEqual({});

    const backgroundOf = (background: Record<string, unknown>) =>
      normalizeDisplay({ styles: { classic: { background } } }).styles;
    expect(backgroundOf({ opacity: -5 })).toEqual({ classic: { background: { opacity: MIN_OPACITY } } });
    expect(backgroundOf({ opacity: 500 })).toEqual({});
    expect(backgroundOf({ opacity: '50' })).toEqual({});
    expect(backgroundOf({ radius: 99 })).toEqual({ classic: { background: { radius: MAX_RADIUS } } });
    expect(backgroundOf({ radius: -2 })).toEqual({ classic: { background: { radius: MIN_RADIUS } } });
    expect(backgroundOf({ radius: 'big' })).toEqual({});
  });

  it('ignores unknown style ids and corrupt style objects', () => {
    expect(normalizeDisplay({ styles: { neon: { elements: { cost: { scale: 150 } } } } }).styles).toEqual({});
    expect(normalizeDisplay({ styles: 'nope' }).styles).toEqual({});
    expect(normalizeDisplay({ styles: [1, 2] }).styles).toEqual({});
    expect(normalizeDisplay({ styles: { halloween: 7 } }).styles).toEqual({});
    expect(normalizeDisplay({ styles: { halloween: null } }).styles).toEqual({});
  });

  it('keeps a tweak for every one of the six elements', () => {
    const stored: Record<string, unknown> = {};
    for (const element of COUNTER_ELEMENTS) stored[element] = { scale: 120 };
    const d = normalizeDisplay({ styles: { classic: { elements: stored } } });
    expect(d.styles.classic?.elements).toEqual(
      Object.fromEntries(COUNTER_ELEMENTS.map((element) => [element, { scale: 120 }])),
    );
  });

  it('drops a stored "time" element, keeping the recognised ones', () => {
    expect(normalizeDisplay({ styles: { classic: { elements: { time: { font: 'mono', scale: 150 } } } } }).styles).toEqual(
      {},
    );
    expect(
      normalizeDisplay({
        styles: { classic: { elements: { time: { font: 'mono' }, elapsed: { font: 'mono' } } } },
      }).styles,
    ).toEqual({ classic: { elements: { elapsed: { font: 'mono' } } } });
  });

  it('drops an override that stores nothing but the preset values', () => {
    const d = normalizeDisplay({
      styles: {
        halloween: {
          elements: {
            label: { font: 'theme', scale: 100, color: null },
            cost: {},
            elapsed: 5,
            status: null,
          },
          background: { mode: 'gradient', direction: 'vertical', opacity: 100 },
        },
      },
    });
    expect(d.styles).toEqual({});
  });

  it('ignores the old per-element settings, with no migration', () => {
    expect(normalizeDisplay({ costFont: 'mono', timeFont: 'serif', costScale: 2, timeScale: 0.6 })).toEqual(
      DEFAULT_DISPLAY,
    );
  });
});

describe('resetStyle', () => {
  it('drops only the tweaks of the selected style', () => {
    const display = normalizeDisplay({
      theme: 'halloween',
      styles: {
        halloween: { elements: { cost: { color: '#ff0000' } } },
        classic: { background: { opacity: 25 } },
      },
    });
    const reset = resetStyle(display);
    expect(reset.theme).toBe('halloween');
    expect(reset.styles).toEqual({ classic: { background: { opacity: 25 } } });
    expect(display.styles.halloween).toBeDefined();
    expect(reset).not.toBe(display);
    expect(reset.mirror).toBe(display.mirror);
  });

  it('is harmless when the style has no tweaks', () => {
    expect(resetStyle(DEFAULT_DISPLAY).styles).toEqual({});
  });
});

describe('camera payload', () => {
  it('sends complete elements and background for a style with no tweaks', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA);
    expect(input.display.elements).toEqual(inheritElements());
    expect(input.display.background).toEqual(styleDefaults(THEMES.classic).background);
  });

  it('sends the stored tweaks instead of the preset', () => {
    const display = normalizeDisplay({
      theme: 'classic',
      styles: {
        classic: {
          elements: { cost: { font: 'mono', scale: 150 } },
          background: { mode: 'solid', color: '#101010' },
        },
      },
    });
    const input = cameraInput(meeting(), display, 'en', DEFAULT_CAMERA);
    expect(input.display.elements.cost).toEqual({ font: 'mono', scale: 150, color: null });
    expect(input.display.elements.elapsed).toEqual(INHERIT);
    expect(input.display.elements.personTime).toEqual(INHERIT);
    expect(input.display.elements.costPerMinute).toEqual(INHERIT);
    expect(input.display.background).toEqual({
      mode: 'solid',
      color: '#101010',
      colorAlt: THEMES.classic.backgroundAlt,
      direction: 'vertical',
      opacity: 100,
      radius: THEMES.classic.radius,
    });
  });

  it('resolves the season in force for the "seasons" style', () => {
    const display: DisplaySettings = { ...DEFAULT_DISPLAY, theme: 'seasons' };
    const autumn = cameraInput(meeting(), display, 'en', DEFAULT_CAMERA, new Date(2026, 9, 22).getTime());
    const winter = cameraInput(meeting(), display, 'en', DEFAULT_CAMERA, new Date(2026, 0, 15).getTime());
    expect(autumn.display.background.color).toBe(THEMES.autumn.background);
    expect(winter.display.background.color).toBe(THEMES.winter.background);
  });
});
