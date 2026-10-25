import { useEffect, useState, type RefObject } from 'react';
import { cameraPreviewFrame } from './platform';

/** `[width u32 LE][height u32 LE]` precede the RGBA pixels (ADR-0035). */
const HEADER_BYTES = 8;
/** About 12 frames a second: smooth enough to place the card, cheap over IPC. */
const POLL_MS = 80;

export interface PreviewFrame {
  width: number;
  height: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

/** Raw IPC answers arrive as an ArrayBuffer, or as a number array on the fallback transport. */
export function toArrayBuffer(answer: ArrayBuffer | number[]): ArrayBuffer {
  return answer instanceof ArrayBuffer ? answer : new Uint8Array(answer).buffer;
}

/** One frame from `camera_preview_frame`, or null when there is none yet or it is malformed. */
export function parsePreviewFrame(buffer: ArrayBuffer): PreviewFrame | null {
  if (buffer.byteLength < HEADER_BYTES) return null;
  const view = new DataView(buffer);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  if (width === 0 || height === 0 || buffer.byteLength !== HEADER_BYTES + width * height * 4) return null;
  return { width, height, rgba: new Uint8ClampedArray(buffer, HEADER_BYTES) };
}

export type PreviewState = { status: 'off' | 'waiting' | 'live' } | { status: 'error'; error: string };

/**
 * Draws the webcam the camera pipeline captures into `canvas` (ADR-0035). The webview never
 * opens the webcam itself: Windows and most Linux drivers allow one consumer, and the pipeline
 * already is it. Failures only change the state; the counter and the virtual camera carry on.
 */
export function useCameraPreview(enabled: boolean, canvas: RefObject<HTMLCanvasElement | null>): PreviewState {
  const [state, setState] = useState<PreviewState>({ status: 'off' });

  useEffect(() => {
    if (!enabled) {
      setState({ status: 'off' });
      return;
    }
    let cancelled = false;
    let reported = '';
    // Re-render only when the state changes, not on every frame.
    const report = (next: PreviewState) => {
      const key = next.status === 'error' ? `error:${next.error}` : next.status;
      if (cancelled || key === reported) return;
      reported = key;
      setState(next);
    };
    report({ status: 'waiting' });

    void (async () => {
      while (!cancelled) {
        try {
          const frame = parsePreviewFrame(toArrayBuffer(await cameraPreviewFrame()));
          const target = canvas.current;
          if (!frame) {
            report({ status: 'waiting' });
          } else if (target && !cancelled) {
            if (target.width !== frame.width) target.width = frame.width;
            if (target.height !== frame.height) target.height = frame.height;
            target.getContext('2d')?.putImageData(new ImageData(frame.rgba, frame.width, frame.height), 0, 0);
            report({ status: 'live' });
          }
        } catch (error) {
          report({ status: 'error', error: String(error) });
        }
        await new Promise((resolve) => setTimeout(resolve, POLL_MS));
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [enabled, canvas]);

  return state;
}
