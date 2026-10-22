import { totalHourlyRate, type Meeting } from './cost';
import { t, type Lang } from './i18n';
import type { DisplaySettings } from './themes';

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
  enabled: boolean;
  width: number;
  height: number;
}

export const DEFAULT_CAMERA: CameraSettings = { enabled: false, width: 1280, height: 720 };

const isSupported = (width: unknown, height: unknown): boolean =>
  CAMERA_SIZES.some((size) => size.width === width && size.height === height);

/** Validates untrusted (stored) camera settings. */
export function normalizeCamera(raw: unknown): CameraSettings {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Record<string, unknown>;
  const supported = isSupported(r.width, r.height);
  return {
    enabled: r.enabled === true,
    width: supported ? (r.width as number) : DEFAULT_CAMERA.width,
    height: supported ? (r.height as number) : DEFAULT_CAMERA.height,
  };
}

export interface CameraLabels {
  totalCost: string;
  elapsed: string;
  costPerMinute: string;
  startsIn: string;
}

export interface CameraDisplay {
  theme: string;
  layout: string;
  costFont: string;
  timeFont: string;
  costScale: number;
  timeScale: number;
  overflow: string;
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
}

const MS_PER_HOUR = 3_600_000;

/** Camera payload for the desktop app, or null before the meeting starts. */
export function cameraInput(
  meeting: Meeting,
  display: DisplaySettings,
  lang: Lang,
  camera: CameraSettings,
): CameraInput | null {
  if (meeting.startAt === null) return null;
  const size = normalizeCamera(camera);
  return {
    startAt: meeting.startAt,
    endAt: meeting.endAt,
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
      layout: display.layout,
      costFont: display.costFont,
      timeFont: display.timeFont,
      costScale: display.costScale,
      timeScale: display.timeScale,
      overflow: display.overflow,
    },
    width: size.width,
    height: size.height,
  };
}
