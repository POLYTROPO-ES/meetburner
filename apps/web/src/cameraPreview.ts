import { useEffect, useState } from 'react';

/** The parts of `MediaDeviceInfo` the matcher needs. */
export interface PreviewDevice {
  kind: string;
  label: string;
  deviceId: string;
}

const normalise = (text: string) => text.trim().toLowerCase().replace(/\s+/g, ' ');

/**
 * The browser's `deviceId` for the camera the user picked in the app's list. Tauri numbers
 * cameras by Media Foundation index and the webview by its own id, so the name is the only
 * thing the two agree on (ADR-0033). `null` when nobody matches: the caller then keeps the
 * default camera instead of failing.
 */
export function matchDeviceId(devices: readonly PreviewDevice[], name: string): string | null {
  const wanted = normalise(name);
  if (wanted.length === 0) return null;
  const cameras = devices.filter((device) => device.kind === 'videoinput' && device.deviceId !== '');
  const exact = cameras.find((device) => normalise(device.label) === wanted);
  if (exact) return exact.deviceId;
  const partial = cameras.find(
    (device) => normalise(device.label).includes(wanted) || wanted.includes(normalise(device.label)),
  );
  return partial ? partial.deviceId : null;
}

export interface CameraPreview {
  stream: MediaStream | null;
  /** Message for the setup page; the counter keeps working whatever happens here. */
  error: string | null;
}

/**
 * Live webcam for the setup preview (ADR-0033). Anything that goes wrong — no permission, a
 * camera another app holds, no device at all — only sets `error`: the card stays editable and
 * the virtual camera untouched, because the preview is a separate capture from the one the
 * Rust side publishes.
 */
export function useCameraPreview(enabled: boolean, label: string | null): CameraPreview {
  const [preview, setPreview] = useState<CameraPreview>({ stream: null, error: null });

  useEffect(() => {
    if (!enabled) {
      setPreview({ stream: null, error: null });
      return;
    }
    const media = typeof navigator === 'undefined' ? undefined : navigator.mediaDevices;
    if (!media || typeof media.getUserMedia !== 'function') {
      setPreview({ stream: null, error: 'no-media' });
      return;
    }

    let cancelled = false;
    let opened: MediaStream | null = null;
    const keep = (stream: MediaStream) => {
      if (cancelled) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      opened = stream;
      setPreview({ stream, error: null });
    };
    const fail = (error: unknown) => {
      if (!cancelled) setPreview({ stream: null, error: String(error) });
    };

    void (async () => {
      try {
        // Asking for any camera first both prompts for permission and unlocks the labels.
        const first = await media.getUserMedia({ video: true });
        const wanted = label ? matchDeviceId(await media.enumerateDevices(), label) : null;
        const current = first.getVideoTracks()[0]?.getSettings().deviceId ?? null;
        if (wanted === null || wanted === current) {
          keep(first);
          return;
        }
        first.getTracks().forEach((track) => track.stop());
        keep(await media.getUserMedia({ video: { deviceId: { exact: wanted } } }));
      } catch (error) {
        fail(error);
      }
    })();

    return () => {
      cancelled = true;
      opened?.getTracks().forEach((track) => track.stop());
    };
  }, [enabled, label]);

  return preview;
}
