import { totalHourlyRate, type Meeting } from './cost';
import { t, type Lang } from './i18n';
import {
  defaultOverlayRect,
  fillsFrame,
  FULL_FRAME,
  resolveTheme,
  styleFor,
  type BackgroundStyle,
  type CounterElement,
  type DisplaySettings,
  type MirrorSettings,
  type OverlayRect,
} from './themes';

/** Aspect ratios offered by the desktop virtual camera (ADR-0015). */
export const CAMERA_RATIOS = ['16:9', '9:16', '4:3', '3:4', '1:1'] as const;
export type CameraRatio = (typeof CAMERA_RATIOS)[number];

export interface CameraSize {
  width: number;
  height: number;
  ratio: CameraRatio;
}

/** Output sizes offered by the desktop virtual camera. Mirrors the Rust allowlist (ADR-0015). */
export const CAMERA_SIZES: readonly CameraSize[] = [
  // 16:9 landscape
  { width: 1920, height: 1080, ratio: '16:9' },
  { width: 1280, height: 720, ratio: '16:9' },
  { width: 640, height: 360, ratio: '16:9' },
  // 9:16 portrait
  { width: 1080, height: 1920, ratio: '9:16' },
  { width: 720, height: 1280, ratio: '9:16' },
  { width: 360, height: 640, ratio: '9:16' },
  // 4:3 landscape
  { width: 1440, height: 1080, ratio: '4:3' },
  { width: 960, height: 720, ratio: '4:3' },
  { width: 640, height: 480, ratio: '4:3' },
  // 3:4 portrait
  { width: 1080, height: 1440, ratio: '3:4' },
  { width: 720, height: 960, ratio: '3:4' },
  { width: 480, height: 640, ratio: '3:4' },
  // 1:1 square
  { width: 1080, height: 1080, ratio: '1:1' },
  { width: 720, height: 720, ratio: '1:1' },
  { width: 480, height: 480, ratio: '1:1' },
];

export interface CameraSettings {
  width: number;
  height: number;
  /** Physical webcam to overlay the counter on (M7); null = none chosen yet. */
  source: string | null;
  /**
   * "Add video background" (ADR-0033). false publishes the counter alone, whatever `source`
   * remembers; the camera then never opens the webcam.
   */
  video: boolean;
}

export const DEFAULT_CAMERA: CameraSettings = { width: 1280, height: 720, source: null, video: false };

const isSupported = (width: unknown, height: unknown): boolean =>
  CAMERA_SIZES.some((size) => size.width === width && size.height === height);

/** Validates untrusted (stored) camera settings. Older settings had no `video`: the source decides. */
export function normalizeCamera(raw: unknown): CameraSettings {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  const supported = isSupported(r.width, r.height);
  const source = typeof r.source === 'string' && r.source.length > 0 ? r.source : null;
  return {
    width: supported ? (r.width as number) : DEFAULT_CAMERA.width,
    height: supported ? (r.height as number) : DEFAULT_CAMERA.height,
    source,
    video: typeof r.video === 'boolean' ? r.video : source !== null,
  };
}

export interface CameraLabels {
  totalCost: string;
  elapsed: string;
  costPerMinute: string;
  startsIn: string;
}

/**
 * The card the camera actually draws (ADR-0032, ADR-0033): the stored rectangle, the built-in
 * corner when the user never moved it, or the whole frame when there is no video behind it.
 */
export function effectiveOverlay(display: DisplaySettings, camera: CameraSettings): OverlayRect {
  const size = normalizeCamera(camera);
  if (!size.video || size.source === null) return FULL_FRAME;
  return display.overlay ?? defaultOverlayRect(size.width, size.height);
}

/** Whether the camera publishes video behind the card. */
export function hasVideoBackground(camera: CameraSettings): boolean {
  const size = normalizeCamera(camera);
  return size.video && size.source !== null;
}

export interface CameraDisplay {
  theme: string;
  layout: string;
  overflow: string;
  mirror: MirrorSettings;
  overlay: OverlayRect | null;
  /** Degrees, 0..360 clockwise; 0 keeps the counter upright. */
  rotation: number;
  /** Complete per-element style: 'theme' keeps the preset font, which the Rust renderer resolves. */
  elements: Record<CounterElement, { font: string; scale: number; color: string | null; visible: boolean }>;
  background: BackgroundStyle;
}

/** Payload of the Rust `camera_start` / `camera_update` commands. */
export interface CameraInput {
  startAt: number;
  endAt: number | null;
  ratePerMs: number;
  currency: string;
  locale: string;
  labels: CameraLabels;
  display: CameraDisplay;
  width: number;
  height: number;
  /** Webcam to overlay the counter on, or null for counter only (M7). */
  source: string | null;
}

const MS_PER_HOUR = 3_600_000;

/**
 * Payload for the desktop camera. It is never null: before the meeting starts the
 * camera publishes an idle frame (00:00:00 / 0,00) so meeting apps can preview it
 * as soon as the app is open (ADR-0018).
 */
export function cameraInput(
  meeting: Meeting,
  display: DisplaySettings,
  lang: Lang,
  camera: CameraSettings,
  nowMs = Date.now(),
): CameraInput {
  const size = normalizeCamera(camera);
  const idle = meeting.startAt === null;
  const style = styleFor(display, resolveTheme(display.theme, new Date(nowMs)));
  const overlay = effectiveOverlay(display, size);
  return {
    startAt: idle ? nowMs : (meeting.startAt as number),
    endAt: idle ? nowMs : meeting.endAt,
    ratePerMs: totalHourlyRate(meeting) / MS_PER_HOUR,
    currency: meeting.currency,
    locale: lang === 'es' ? 'es-ES' : 'en-US',
    labels: {
      totalCost: t(lang, 'totalCost'),
      elapsed: t(lang, 'elapsed'),
      costPerMinute: t(lang, 'costPerMinute'),
      startsIn: t(lang, 'startsIn'),
    },
    display: {
      theme: display.theme,
      // Covering the frame is the fullscreen layout; anything smaller is a card (ADR-0032).
      layout: fillsFrame(overlay) ? 'fullscreen' : 'widget',
      overflow: display.overflow,
      mirror: display.mirror,
      overlay,
      rotation: style.rotation,
      elements: style.elements,
      background: style.background,
    },
    width: size.width,
    height: size.height,
    // The webcam only opens when the user asked for a video background (ADR-0033).
    source: size.video ? size.source : null,
  };
}
