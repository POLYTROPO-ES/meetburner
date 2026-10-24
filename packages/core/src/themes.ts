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
  christmas: { background: '#0b3d20', backgroundAlt: '#7a0f1a', text: '#fffaf0', muted: '#c9dcc8', accent: '#ffd34e', font: 'serif', weight: 700, radius: 16, decoration: '' },
  minimalist: { background: '#ffffff', backgroundAlt: '#ffffff', text: '#111111', muted: '#767676', accent: '#111111', font: 'system', weight: 300, radius: 0, decoration: '' },
  spring: { background: '#f0fdf4', backgroundAlt: '#fce7f3', text: '#14532d', muted: '#4c7055', accent: '#db2777', font: 'rounded', weight: 700, radius: 20, decoration: '' },
  summer: { background: '#fff7d6', backgroundAlt: '#7dd3fc', text: '#0c4a6e', muted: '#34566a', accent: '#c2410c', font: 'rounded', weight: 800, radius: 20, decoration: '' },
  autumn: { background: '#2b1a0e', backgroundAlt: '#7c2d12', text: '#fef3c7', muted: '#d6b98c', accent: '#f59e0b', font: 'serif', weight: 700, radius: 16, decoration: '' },
  winter: { background: '#e0f2fe', backgroundAlt: '#f8fafc', text: '#0f172a', muted: '#5a6b83', accent: '#0284c7', font: 'system', weight: 700, radius: 16, decoration: '' },
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

export type CounterElement = 'label' | 'cost' | 'status' | 'elapsed' | 'personTime' | 'costPerMinute';
export const COUNTER_ELEMENTS: readonly CounterElement[] = [
  'label',
  'cost',
  'status',
  'elapsed',
  'personTime',
  'costPerMinute',
];

/** Per-element tweaks. `null` / 'theme' keep whatever the preset defines. */
export interface ElementStyle {
  font: FontChoice;
  /** 50..250 percent, 100 = the preset size. */
  scale: number;
  color: string | null;
  /** false hides the element in the counter and on the camera. */
  visible: boolean;
}

export type BackgroundMode = 'solid' | 'gradient';
export type GradientDirection = 'horizontal' | 'vertical';
export const BACKGROUND_MODES: readonly BackgroundMode[] = ['solid', 'gradient'];
export const GRADIENT_DIRECTIONS: readonly GradientDirection[] = ['horizontal', 'vertical'];

export interface BackgroundStyle {
  mode: BackgroundMode;
  color: string;
  colorAlt: string;
  direction: GradientDirection;
  /** 0 = fully transparent, 100 = solid. */
  opacity: number;
  /** 0..40 px, 0 = square corners. */
  radius: number;
}

/** The tweaks stored for one style; `styleDefaults`/`styleFor` fill whatever is missing. */
export interface StyleOverride {
  elements: Record<CounterElement, ElementStyle>;
  background: BackgroundStyle;
  /** Degrees, 0..360, clockwise. 0 keeps the counter upright. */
  rotation: number;
}

export interface DisplaySettings {
  theme: ThemeId;
  /** Desktop camera: widget when overlaid on a webcam, fullscreen otherwise. */
  layout: Layout;
  overflow: Overflow;
  mirror: MirrorSettings;
  /** Where the widget card sits on the camera frame; null keeps the built-in corner (ADR-0023). */
  overlay: OverlayRect | null;
  /** Customisation per style id, so each style keeps its own look; absent = the preset (ADR-0029). */
  styles: Partial<Record<ThemeId, StyleOverride>>;
}

/** Widget card rectangle, as fractions of the camera frame (0..1). */
export interface OverlayRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Smallest card the editor allows, as a fraction of the frame. */
export const OVERLAY_MIN = 0.15;

/** Same widget geometry as the Rust renderer, for the sizes it does not know yet. */
export function defaultOverlayRect(width: number, height: number): OverlayRect {
  const margin = 32;
  const w = 0.42;
  const h = 0.3;
  return { x: 1 - margin / width - w, y: 1 - margin / height - h, w, h };
}

const within = (value: number, max: number): number => Math.min(max, Math.max(0, value));

/** Validates a stored rectangle: fractions, minimum size, inside the frame. */
export function clampOverlay(rect: OverlayRect): OverlayRect {
  const w = Math.min(1, Math.max(OVERLAY_MIN, rect.w));
  const h = Math.min(1, Math.max(OVERLAY_MIN, rect.h));
  return { x: within(rect.x, 1 - w), y: within(rect.y, 1 - h), w, h };
}

/** Moves the card, stopping at the frame edge without shrinking it. */
export function moveOverlay(rect: OverlayRect, dx: number, dy: number): OverlayRect {
  return {
    ...rect,
    x: within(rect.x + dx, 1 - rect.w),
    y: within(rect.y + dy, 1 - rect.h),
  };
}

/** Grows or shrinks the card from its bottom-right corner. */
export function resizeOverlay(rect: OverlayRect, dw: number, dh: number): OverlayRect {
  return {
    ...rect,
    w: Math.min(1 - rect.x, Math.max(OVERLAY_MIN, rect.w + dw)),
    h: Math.min(1 - rect.y, Math.max(OVERLAY_MIN, rect.h + dh)),
  };
}

/**
 * Desktop camera only: the captured webcam and the counter are mirrored independently, so a
 * mirrored self-view never turns the numbers into unreadable text (ADR-0022).
 */
export interface MirrorSettings {
  sourceH: boolean;
  sourceV: boolean;
  overlayH: boolean;
  overlayV: boolean;
}

export const NO_MIRROR: MirrorSettings = {
  sourceH: false,
  sourceV: false,
  overlayH: false,
  overlayV: false,
};

export const MIN_SCALE = 0.5;
export const MAX_SCALE = 2.5;
export const MIN_RADIUS = 0;
export const MAX_RADIUS = 40;
export const MIN_OPACITY = 0;
export const MAX_OPACITY = 100;
export const MIN_ROTATION = 0;
export const MAX_ROTATION = 360;

/** `ElementStyle.scale` is a percentage; MIN_SCALE..MAX_SCALE is the multiplier the renderer applies. */
const MIN_ELEMENT_SCALE = MIN_SCALE * 100;
const MAX_ELEMENT_SCALE = MAX_SCALE * 100;

/** The radius comes from each preset: `styleDefaults(theme).background.radius`. */
export const DEFAULT_DISPLAY: DisplaySettings = {
  theme: 'classic',
  layout: 'fullscreen',
  overflow: 'compact',
  mirror: NO_MIRROR,
  overlay: null,
  styles: {},
};

const oneOf = <T extends string>(value: unknown, allowed: readonly T[], fallback: T): T =>
  allowed.includes(value as T) ? (value as T) : fallback;

const clampNumber = (value: unknown, min: number, max: number, fallback: number): number =>
  typeof value === 'number' && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;

const FONT_CHOICES: readonly FontChoice[] = ['theme', ...FONT_IDS];

const HEX_COLOR = /^#[0-9a-f]{3}$|^#[0-9a-f]{6}$/i;
const isHex = (value: unknown): value is string => typeof value === 'string' && HEX_COLOR.test(value);

/** Validates untrusted (stored) display settings. Old `costFont`/`costScale` keys are ignored (ADR-0029). */
export function normalizeDisplay(raw: unknown): DisplaySettings {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  const mirror = (r.mirror && typeof r.mirror === 'object' ? r.mirror : {}) as Record<string, unknown>;
  const flag = (value: unknown): boolean => value === true;
  return {
    theme: oneOf(r.theme, THEME_IDS, DEFAULT_DISPLAY.theme),
    layout: oneOf<Layout>(r.layout, ['fullscreen', 'widget'], DEFAULT_DISPLAY.layout),
    overflow: oneOf(r.overflow, OVERFLOW_MODES, DEFAULT_DISPLAY.overflow),
    mirror: {
      sourceH: flag(mirror.sourceH),
      sourceV: flag(mirror.sourceV),
      overlayH: flag(mirror.overlayH),
      overlayV: flag(mirror.overlayV),
    },
    overlay: normalizeOverlay(r.overlay),
    styles: normalizeStyles(r.styles),
  };
}

/** A stored override may be partial: written by an older build, or hand-edited in devtools. */
interface StoredStyle {
  elements?: Partial<Record<CounterElement, ElementStyle>>;
  background?: Partial<BackgroundStyle>;
  rotation?: number;
}

/** Only known style ids survive, and each one drops whatever equals the preset defaults. */
function normalizeStyles(raw: unknown): Partial<Record<ThemeId, StyleOverride>> {
  const styles: Partial<Record<ThemeId, StyleOverride>> = {};
  if (!raw || typeof raw !== 'object') return styles;
  const source = raw as Record<string, unknown>;
  for (const id of THEME_IDS) {
    const style = normalizeStyle(source[id]);
    // Partial on purpose: `styleFor` completes it from the preset of the theme in force.
    if (style) styles[id] = style as StyleOverride;
  }
  return styles;
}

function normalizeStyle(raw: unknown): StoredStyle | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Record<string, unknown>;
  const source = (r.elements && typeof r.elements === 'object' ? r.elements : {}) as Record<string, unknown>;
  const elements: Partial<Record<CounterElement, ElementStyle>> = {};
  for (const element of COUNTER_ELEMENTS) {
    const value = source[element];
    if (!value || typeof value !== 'object') continue;
    const e = value as Record<string, unknown>;
    const style: Partial<ElementStyle> = {};
    const font = oneOf(e.font, FONT_CHOICES, 'theme');
    if (font !== 'theme') style.font = font;
    const elementScale = clampNumber(e.scale, MIN_ELEMENT_SCALE, MAX_ELEMENT_SCALE, 100);
    if (elementScale !== 100) style.scale = elementScale;
    if (isHex(e.color)) style.color = e.color;
    // Only a hidden element is a difference: missing/junk visibility means visible.
    if (e.visible === false) style.visible = false;
    if (Object.keys(style).length > 0) elements[element] = style as ElementStyle;
  }
  const background = normalizeBackground(r.background);
  const stored: StoredStyle = {};
  if (Object.keys(elements).length > 0) stored.elements = elements;
  if (background) stored.background = background;
  const rotation = clampNumber(r.rotation, MIN_ROTATION, MAX_ROTATION, MIN_ROTATION);
  if (rotation !== MIN_ROTATION) stored.rotation = rotation;
  return Object.keys(stored).length > 0 ? stored : null;
}

function normalizeBackground(raw: unknown): Partial<BackgroundStyle> | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Record<string, unknown>;
  // Colours are either absent (keep the preset's) or a valid hex; anything else drops the object.
  if ((r.color !== undefined && !isHex(r.color)) || (r.colorAlt !== undefined && !isHex(r.colorAlt))) return null;
  const background: Partial<BackgroundStyle> = {};
  if (isHex(r.color)) background.color = r.color;
  if (isHex(r.colorAlt)) background.colorAlt = r.colorAlt;
  const mode = oneOf(r.mode, BACKGROUND_MODES, 'gradient');
  if (mode !== 'gradient') background.mode = mode;
  const direction = oneOf(r.direction, GRADIENT_DIRECTIONS, 'vertical');
  if (direction !== 'vertical') background.direction = direction;
  const opacity = clampNumber(r.opacity, MIN_OPACITY, MAX_OPACITY, MAX_OPACITY);
  if (opacity !== MAX_OPACITY) background.opacity = opacity;
  // Any finite radius is a real tweak, 0 (square corners) included: the preset radius is unknown here.
  if (typeof r.radius === 'number' && Number.isFinite(r.radius)) {
    background.radius = clampNumber(r.radius, MIN_RADIUS, MAX_RADIUS, MIN_RADIUS);
  }
  return Object.keys(background).length > 0 ? background : null;
}

/** The preset values, used when a style has no stored tweaks. */
export function styleDefaults(theme: Theme): StyleOverride {
  const elements = {} as Record<CounterElement, ElementStyle>;
  for (const element of COUNTER_ELEMENTS) {
    elements[element] = { font: 'theme', scale: 100, color: null, visible: true };
  }
  return {
    elements,
    background: {
      mode: 'gradient',
      color: theme.background,
      colorAlt: theme.backgroundAlt,
      direction: 'vertical',
      opacity: MAX_OPACITY,
      radius: theme.radius,
    },
    rotation: MIN_ROTATION,
  };
}

/**
 * Stored tweaks for `display.theme`, completed from the preset. `theme` is the already-resolved
 * theme, so `seasons` gets the season in force.
 */
export function styleFor(display: DisplaySettings, theme: Theme): StyleOverride {
  const defaults = styleDefaults(theme);
  const stored = (display.styles ? display.styles[display.theme] : undefined) as StoredStyle | undefined;
  if (!stored || typeof stored !== 'object') return defaults;
  const elements = {} as Record<CounterElement, ElementStyle>;
  for (const element of COUNTER_ELEMENTS) {
    const raw = stored.elements ? stored.elements[element] : undefined;
    const saved = raw && typeof raw === 'object' ? raw : undefined;
    const fallback = defaults.elements[element];
    elements[element] = {
      font: saved ? oneOf(saved.font, FONT_CHOICES, 'theme') : fallback.font,
      scale: saved ? clampNumber(saved.scale, MIN_ELEMENT_SCALE, MAX_ELEMENT_SCALE, 100) : fallback.scale,
      color: saved && isHex(saved.color) ? saved.color : null,
      visible: saved ? saved.visible !== false : fallback.visible,
    };
  }
  const bg = (
    stored.background && typeof stored.background === 'object' ? stored.background : {}
  ) as Partial<BackgroundStyle>;
  return {
    elements,
    rotation: clampNumber(stored.rotation, MIN_ROTATION, MAX_ROTATION, MIN_ROTATION),
    background: {
      mode: oneOf(bg.mode, BACKGROUND_MODES, defaults.background.mode),
      color: isHex(bg.color) ? bg.color : defaults.background.color,
      colorAlt: isHex(bg.colorAlt) ? bg.colorAlt : defaults.background.colorAlt,
      direction: oneOf(bg.direction, GRADIENT_DIRECTIONS, defaults.background.direction),
      opacity: clampNumber(bg.opacity, MIN_OPACITY, MAX_OPACITY, MAX_OPACITY),
      radius: clampNumber(bg.radius, MIN_RADIUS, MAX_RADIUS, defaults.background.radius),
    },
  };
}

/** Drops the tweaks of the selected style, restoring the preset. */
export function resetStyle(display: DisplaySettings): DisplaySettings {
  const styles = { ...display.styles };
  delete styles[display.theme];
  return { ...display, styles };
}

function normalizeOverlay(raw: unknown): OverlayRect | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Record<string, unknown>;
  const numbers = [r.x, r.y, r.w, r.h];
  if (!numbers.every((value) => typeof value === 'number' && Number.isFinite(value))) return null;
  return clampOverlay({ x: r.x as number, y: r.y as number, w: r.w as number, h: r.h as number });
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
