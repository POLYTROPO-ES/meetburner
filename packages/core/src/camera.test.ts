import { describe, expect, it } from 'vitest';
import {
  CAMERA_RATIOS,
  CAMERA_SIZES,
  cameraInput,
  DEFAULT_CAMERA,
  effectiveOverlay,
  normalizeCamera,
} from './camera';
import { createMeeting, totalHourlyRate } from './cost';
import { DEFAULT_DISPLAY, FULL_FRAME, THEMES } from './themes';

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

describe('camera settings', () => {
  it('only keeps the supported sizes', () => {
    for (const size of CAMERA_SIZES) {
      expect(normalizeCamera({ width: size.width, height: size.height })).toEqual({
        width: size.width,
        height: size.height,
        source: null,
      });
    }
    expect(normalizeCamera({ width: 500, height: 500 })).toEqual(DEFAULT_CAMERA);
    expect(normalizeCamera(null)).toEqual(DEFAULT_CAMERA);
    expect(normalizeCamera('nope')).toEqual(DEFAULT_CAMERA);
  });

  it('remembers the webcam chosen for the overlay', () => {
    expect(normalizeCamera({ width: 1280, height: 720, source: '1' }).source).toBe('1');
    expect(normalizeCamera({ width: 1280, height: 720, source: '' }).source).toBeNull();
    expect(normalizeCamera({ width: 1280, height: 720, source: 7 }).source).toBeNull();
  });

  it('offers a default size from the supported list', () => {
    expect(CAMERA_SIZES).toContainEqual({
      width: DEFAULT_CAMERA.width,
      height: DEFAULT_CAMERA.height,
      ratio: '16:9',
    });
  });

  it('offers every ratio in its natural orientation', () => {
    expect([...CAMERA_RATIOS]).toEqual(['16:9', '9:16', '4:3', '3:4', '1:1']);
    for (const ratio of CAMERA_RATIOS) {
      expect(CAMERA_SIZES.some((size) => size.ratio === ratio)).toBe(true);
    }
    const landscape = CAMERA_SIZES.filter((size) => size.width > size.height).map((size) => size.ratio);
    const portrait = CAMERA_SIZES.filter((size) => size.width < size.height).map((size) => size.ratio);
    const square = CAMERA_SIZES.filter((size) => size.width === size.height).map((size) => size.ratio);
    expect(landscape).toEqual(expect.arrayContaining(['16:9', '4:3']));
    expect(portrait).toEqual(expect.arrayContaining(['9:16', '3:4']));
    expect(square).toContain('1:1');
  });

  it('keeps the exact ratio of every entry', () => {
    for (const size of CAMERA_SIZES) {
      const [width, height] = size.ratio.split(':').map(Number);
      expect(size.width * height).toBe(size.height * width);
    }
  });

  it('accepts every offered size, portrait and square included', () => {
    for (const size of CAMERA_SIZES) {
      expect(normalizeCamera({ width: size.width, height: size.height })).toEqual({
        width: size.width,
        height: size.height,
        source: null,
      });
    }
  });
});

describe('camera input', () => {
  it('publishes an idle frame before the meeting starts', () => {
    const idle = cameraInput(
      { ...createMeeting(), startAt: null },
      DEFAULT_DISPLAY,
      'en',
      DEFAULT_CAMERA,
      5_000,
    );
    expect(idle.startAt).toBe(5_000);
    expect(idle.endAt).toBe(5_000);
    expect(idle.ratePerMs).toBe(0);
  });

  it('sends the combined rate per millisecond and the meeting window', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA);
    expect(input.ratePerMs).toBeCloseTo(totalHourlyRate(meeting()) / 3_600_000, 12);
    expect(input.startAt).toBe(1_700_000_000_000);
    expect(input.endAt).toBeNull();
    expect(input.currency).toBe('EUR');
    expect(input.locale).toBe('en-US');
  });

  it('translates the labels', () => {
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA).labels).toEqual({
      totalCost: 'Total cost',
      elapsed: 'Elapsed',
      costPerMinute: 'Cost per minute',
      startsIn: 'Starts in',
    });
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'es', DEFAULT_CAMERA).labels.totalCost).toBe('Coste total');
  });

  it('passes the display settings through, with the style resolved', () => {
    const display = {
      ...DEFAULT_DISPLAY,
      theme: 'halloween' as const,
      overflow: 'shrink' as const,
    };
    expect(cameraInput(meeting(), display, 'en', DEFAULT_CAMERA).display).toEqual({
      theme: 'halloween',
      // No webcam chosen, so the card covers the frame and the renderer goes fullscreen.
      layout: 'fullscreen',
      overflow: 'shrink',
      mirror: { sourceH: false, sourceV: false, overlayH: false, overlayV: false },
      overlay: FULL_FRAME,
      rotation: 0,
      elements: {
        label: { font: 'theme', scale: 100, color: null, visible: true },
        cost: { font: 'theme', scale: 100, color: null, visible: true },
        status: { font: 'theme', scale: 100, color: null, visible: true },
        elapsed: { font: 'theme', scale: 100, color: null, visible: true },
        personTime: { font: 'theme', scale: 100, color: null, visible: true },
        costPerMinute: { font: 'theme', scale: 100, color: null, visible: true },
      },
      background: {
        mode: 'gradient',
        color: THEMES.halloween.background,
        colorAlt: THEMES.halloween.backgroundAlt,
        direction: 'vertical',
        opacity: 100,
        radius: THEMES.halloween.radius,
      },
    });
  });

  it('sends the selected output size', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', {
      width: 1920,
      height: 1080,
      source: null,
    });
    expect({ width: input.width, height: input.height }).toEqual({ width: 1920, height: 1080 });
  });

  it('sends the webcam to overlay on', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', {
      width: 1280,
      height: 720,
      source: '2',
    });
    expect(input.source).toBe('2');
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA).source).toBeNull();
  });
});

describe('the card the camera draws', () => {
  const webcam = { width: 1280, height: 720, source: '1' };

  it('covers the frame when there is no webcam to sit on', () => {
    expect(effectiveOverlay(DEFAULT_DISPLAY, DEFAULT_CAMERA)).toEqual(FULL_FRAME);
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA).display.layout).toBe('fullscreen');
  });

  it('falls back to the built-in corner once a webcam is chosen', () => {
    const card = effectiveOverlay(DEFAULT_DISPLAY, webcam);
    expect(card.w).toBeCloseTo(0.42, 5);
    expect(card.h).toBeCloseTo(0.3, 5);
    expect(card.x + card.w).toBeCloseTo(1 - 32 / 1280, 5);
    expect(card.y + card.h).toBeCloseTo(1 - 32 / 720, 5);
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', webcam).display.layout).toBe('widget');
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', webcam).display.overlay).toEqual(card);
  });

  it('turns the fullscreen layout on and off with the card size', () => {
    const stored = (overlay: { x: number; y: number; w: number; h: number }) => ({
      ...DEFAULT_DISPLAY,
      overlay,
    });
    const full = cameraInput(meeting(), stored(FULL_FRAME), 'en', webcam);
    expect(full.display.layout).toBe('fullscreen');
    // A card that falls short of the frame by a visible margin is a card.
    const card = cameraInput(meeting(), stored({ x: 0, y: 0, w: 0.9, h: 1 }), 'en', webcam);
    expect(card.display.layout).toBe('widget');
    expect(card.display.overlay).toEqual({ x: 0, y: 0, w: 0.9, h: 1 });
    // Rounding from the drag gestures must not flip the mode.
    const almost = cameraInput(meeting(), stored({ x: 0, y: 0, w: 0.999, h: 0.999 }), 'en', webcam);
    expect(almost.display.layout).toBe('fullscreen');
  });
});
