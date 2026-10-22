import { totalHourlyRate, type Meeting } from './cost';
import { t, type Lang } from './i18n';
import type { DisplaySettings } from './themes';

/** Output sizes offered by the desktop virtual camera. Mirrors the Rust allowlist (ADR-0015). */
export const CAMERA_SIZES = [
  { width: 640, height: 360 },
  { width: 1280, height: 720 },
  { width: 1920, height: 1080 },
] as const;

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
