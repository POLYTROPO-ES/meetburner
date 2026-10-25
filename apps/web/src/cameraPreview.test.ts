import { describe, expect, it } from 'vitest';
import { matchDeviceId, type PreviewDevice } from './cameraPreview';

const camera = (deviceId: string, label: string): PreviewDevice => ({ kind: 'videoinput', deviceId, label });

describe('finding the webview camera that matches the app list', () => {
  const devices: PreviewDevice[] = [
    { kind: 'audioinput', deviceId: 'mic', label: 'Desk microphone' },
    camera('cam1', 'HD Pro Webcam C920'),
    camera('cam2', 'Integrated Camera'),
    camera('cam3', 'MeetBurner Camera'),
  ];

  it('matches the label exactly', () => {
    expect(matchDeviceId(devices, 'Integrated Camera')).toBe('cam2');
  });

  it('ignores case and extra spaces', () => {
    expect(matchDeviceId(devices, '  integrated   camera ')).toBe('cam2');
  });

  it('falls back to a partial match, which is what drivers add suffixes for', () => {
    expect(matchDeviceId(devices, 'HD Pro Webcam C920 (1)')).toBe('cam1');
    expect(matchDeviceId(devices, 'c920')).toBe('cam1');
  });

  it('never returns an audio device or an empty id', () => {
    expect(matchDeviceId(devices, 'Desk microphone')).toBeNull();
    expect(matchDeviceId([camera('', 'Integrated Camera')], 'Integrated Camera')).toBeNull();
  });

  it('returns nothing when no camera matches, so the caller keeps the default one', () => {
    expect(matchDeviceId(devices, 'Front Camera')).toBeNull();
    expect(matchDeviceId(devices, '   ')).toBeNull();
    expect(matchDeviceId([], 'Integrated Camera')).toBeNull();
  });
});
