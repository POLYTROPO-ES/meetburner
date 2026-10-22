export const THEME_IDS = ['classic', 'modern', 'formal', 'halloween', 'christmas', 'seasons', 'minimalist'] as const;
export type ThemeId = (typeof THEME_IDS)[number];
export type Season = 'spring' | 'summer' | 'autumn' | 'winter';

export const FONT_IDS = ['system', 'serif', 'mono', 'rounded', 'condensed', 'handwritten', 'display'] as const;
export type FontId = (typeof FONT_IDS)[number];

export const FONTS: Record<FontId, { stack: string }> = {
  system: { stack: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif" },
  serif: { stack: "Georgia, 'Times New Roman', serif" },
  mono: { stack: "'Cascadia Mono', Consolas, 'SF Mono', Menlo, monospace" },
  rounded: { stack: "ui-rounded, 'Arial Rounded MT Bold', Nunito, system-ui, sans-serif" },
  condensed: { stack: "Bahnschrift, 'Roboto Condensed', 'Arial Narrow', sans-serif" },
  handwritten: { stack: "'Segoe Print', 'Comic Sans MS', 'Chalkboard SE', cursive" },
  display: { stack: "Impact, Haettenschweiler, 'Arial Black', sans-serif" },
};

/** Plain data (hex colours, font ids) so the Rust camera renderer can use the same themes. */
export interface Theme {
  background: string;
  backgroundAlt: string;
  text: string;
  muted: string;
  accent: string;
  font: FontId;
  weight: number;
  radius: number;
  decoration: string;
}

export const THEMES: Record<Exclude<ThemeId, 'seasons'> | Season, Theme> = {
  classic: { background: '#0f1115', backgroundAlt: '#181b22', text: '#e8eaf0', muted: '#8b93a7', accent: '#ff5d5d', font: 'system', weight: 700, radius: 12, decoration: '' },
  modern: { background: '#0b1020', backgroundAlt: '#1d2b64', text: '#f5f7ff', muted: '#9aa6d1', accent: '#4ade80', font: 'rounded', weight: 800, radius: 20, decoration: '' },
  formal: { background: '#f7f5f0', backgroundAlt: '#e9e4d8', text: '#1c1c1c', muted: '#6b6457', accent: '#1f3a5f', font: 'serif', weight: 600, radius: 4, decoration: '' },
  halloween: { background: '#1a0f05', backgroundAlt: '#3d1d00', text: '#ffedd5', muted: '#c2a383', accent: '#ff7518', font: 'display', weight: 700, radius: 16, decoration: '🎃🦇' },
  christmas: { background: '#0b3d20', backgroundAlt: '#7a0f1a', text: '#fffaf0', muted: '#c9dcc8', accent: '#ffd34e', font: 'serif', weight: 700, radius: 16, decoration: '🎄❄️' },
  minimalist: { background: '#ffffff', backgroundAlt: '#ffffff', text: '#111111', muted: '#8a8a8a', accent: '#111111', font: 'system', weight: 300, radius: 0, decoration: '' },
  spring: { background: '#f0fdf4', backgroundAlt: '#fce7f3', text: '#14532d', muted: '#6b8f71', accent: '#db2777', font: 'rounded', weight: 700, radius: 20, decoration: '🌸' },
  summer: { background: '#fff7d6', backgroundAlt: '#7dd3fc', text: '#0c4a6e', muted: '#5b7f95', accent: '#f97316', font: 'rounded', weight: 800, radius: 20, decoration: '☀️' },
  autumn: { background: '#2b1a0e', backgroundAlt: '#7c2d12', text: '#fef3c7', muted: '#d6b98c', accent: '#f59e0b', font: 'serif', weight: 700, radius: 16, decoration: '🍂' },
  winter: { background: '#e0f2fe', backgroundAlt: '#f8fafc', text: '#0f172a', muted: '#64748b', accent: '#0284c7', font: 'system', weight: 700, radius: 16, decoration: '❄️' },
};

/** Meteorological seasons, northern hemisphere. */
export function seasonOf(date: Date): Season {
  const m = date.getMonth();
  if (m >= 2 && m <= 4) return 'spring';
  if (m >= 5 && m <= 7) return 'summer';
  if (m >= 8 && m <= 10) return 'autumn';
  return 'winter';
}

export function resolveTheme(id: ThemeId, date: Date): Theme {
  return id === 'seasons' ? THEMES[seasonOf(date)] : THEMES[id];
}

export type Layout = 'fullscreen' | 'widget';
export type FontChoice = FontId | 'theme';
/** What to do when the cost is too wide for the counter: abbreviate (K, M, B) or shrink the font. */
export type Overflow = 'compact' | 'shrink';
export const OVERFLOW_MODES: readonly Overflow[] = ['compact', 'shrink'];

export interface DisplaySettings {
  theme: ThemeId;
  /** Desktop camera: widget when overlaid on a webcam, fullscreen otherwise. */
  layout: Layout;
  costFont: FontChoice;
  timeFont: FontChoice;
  costScale: number;
  timeScale: number;
  overflow: Overflow;
}

export const MIN_SCALE = 0.5;
export const MAX_SCALE = 2.5;

export const DEFAULT_DISPLAY: DisplaySettings = {
  theme: 'classic',
  layout: 'fullscreen',
  costFont: 'theme',
  timeFont: 'theme',
  costScale: 1,
  timeScale: 1,
  overflow: 'compact',
};

const oneOf = <T extends string>(value: unknown, allowed: readonly T[], fallback: T): T =>
  allowed.includes(value as T) ? (value as T) : fallback;

const scale = (value: unknown): number =>
  typeof value === 'number' && Number.isFinite(value) ? Math.min(MAX_SCALE, Math.max(MIN_SCALE, value)) : 1;

const FONT_CHOICES: readonly FontChoice[] = ['theme', ...FONT_IDS];

/** Validates untrusted (stored) display settings. */
export function normalizeDisplay(raw: unknown): DisplaySettings {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  return {
    theme: oneOf(r.theme, THEME_IDS, DEFAULT_DISPLAY.theme),
    layout: oneOf<Layout>(r.layout, ['fullscreen', 'widget'], DEFAULT_DISPLAY.layout),
    costFont: oneOf(r.costFont, FONT_CHOICES, 'theme'),
    timeFont: oneOf(r.timeFont, FONT_CHOICES, 'theme'),
    costScale: scale(r.costScale),
    timeScale: scale(r.timeScale),
    overflow: oneOf(r.overflow, OVERFLOW_MODES, DEFAULT_DISPLAY.overflow),
  };
}

/** Decides how to show a cost whose full/abbreviated texts are `fullWidth`/`compactWidth` px in an `availableWidth` px box. */
export function fitCost(
  availableWidth: number,
  fullWidth: number,
  compactWidth: number,
  mode: Overflow,
): { compact: boolean; scale: number } {
  const shrink = (width: number) => (width <= availableWidth ? 1 : Math.floor((availableWidth / width) * 100) / 100);
  if (fullWidth <= availableWidth || availableWidth <= 0) return { compact: false, scale: 1 };
  if (mode === 'compact') return { compact: true, scale: shrink(compactWidth) };
  return { compact: false, scale: shrink(fullWidth) };
}

export function resolveFont(choice: FontChoice, theme: Theme): string {
  return FONTS[choice === 'theme' ? theme.font : choice].stack;
}
