import { describe, expect, it } from 'vitest';
import { CAMERA_RATIOS, CAMERA_SIZES, cameraInput, DEFAULT_CAMERA, normalizeCamera } from './camera';
import { createMeeting, totalHourlyRate } from './cost';
import { DEFAULT_DISPLAY, THEMES } from './themes';

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
      layout: 'widget' as const,
      overflow: 'shrink' as const,
    };
    expect(cameraInput(meeting(), display, 'en', DEFAULT_CAMERA).display).toEqual({
      theme: 'halloween',
      layout: 'widget',
      overflow: 'shrink',
      mirror: { sourceH: false, sourceV: false, overlayH: false, overlayV: false },
      overlay: null,
      elements: {
        label: { font: 'theme', scale: 100, color: null },
        cost: { font: 'theme', scale: 100, color: null },
        time: { font: 'theme', scale: 100, color: null },
        status: { font: 'theme', scale: 100, color: null },
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
