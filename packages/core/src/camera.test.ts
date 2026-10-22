import { describe, expect, it } from 'vitest';
import { CAMERA_RATIOS, CAMERA_SIZES, cameraInput, DEFAULT_CAMERA, normalizeCamera } from './camera';
import { createMeeting, totalHourlyRate } from './cost';
import { DEFAULT_DISPLAY } from './themes';

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
      expect(normalizeCamera({ enabled: true, width: size.width, height: size.height })).toEqual({
        enabled: true,
        width: size.width,
        height: size.height,
      });
    }
    expect(normalizeCamera({ enabled: true, width: 500, height: 500 })).toEqual({
      ...DEFAULT_CAMERA,
      enabled: true,
    });
    expect(normalizeCamera(null)).toEqual(DEFAULT_CAMERA);
    expect(normalizeCamera('nope')).toEqual(DEFAULT_CAMERA);
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
      expect(normalizeCamera({ enabled: true, width: size.width, height: size.height })).toEqual({
        enabled: true,
        width: size.width,
        height: size.height,
      });
    }
  });
});

describe('camera input', () => {
  it('is null until the meeting starts', () => {
    expect(cameraInput({ ...createMeeting(), startAt: null }, DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA)).toBeNull();
  });

  it('sends the combined rate per millisecond and the meeting window', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA);
    expect(input).not.toBeNull();
    expect(input!.ratePerMs).toBeCloseTo(totalHourlyRate(meeting()) / 3_600_000, 12);
    expect(input!.startAt).toBe(1_700_000_000_000);
    expect(input!.endAt).toBeNull();
    expect(input!.currency).toBe('EUR');
    expect(input!.locale).toBe('en-US');
  });

  it('translates the labels', () => {
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'en', DEFAULT_CAMERA)!.labels).toEqual({
      totalCost: 'Total cost',
      elapsed: 'Elapsed',
      costPerMinute: 'Cost per minute',
      startsIn: 'Starts in',
    });
    expect(cameraInput(meeting(), DEFAULT_DISPLAY, 'es', DEFAULT_CAMERA)!.labels.totalCost).toBe('Coste total');
  });

  it('passes the display settings through', () => {
    const display = {
      ...DEFAULT_DISPLAY,
      theme: 'halloween' as const,
      layout: 'widget' as const,
      costFont: 'mono' as const,
      timeFont: 'serif' as const,
      costScale: 1.5,
      timeScale: 0.8,
      overflow: 'shrink' as const,
    };
    expect(cameraInput(meeting(), display, 'en', DEFAULT_CAMERA)!.display).toEqual({
      theme: 'halloween',
      layout: 'widget',
      costFont: 'mono',
      timeFont: 'serif',
      costScale: 1.5,
      timeScale: 0.8,
      overflow: 'shrink',
    });
  });

  it('sends the selected output size', () => {
    const input = cameraInput(meeting(), DEFAULT_DISPLAY, 'en', {
      enabled: true,
      width: 1920,
      height: 1080,
    });
    expect({ width: input!.width, height: input!.height }).toEqual({ width: 1920, height: 1080 });
  });
});
