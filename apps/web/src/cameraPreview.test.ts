import { describe, expect, it } from 'vitest';
import { parsePreviewFrame, toArrayBuffer } from './cameraPreview';

/** `[width u32 LE][height u32 LE]` then RGBA, as `camera_preview_frame` sends it. */
function payload(width: number, height: number, pixels: number[]): ArrayBuffer {
  const bytes = new Uint8Array(8 + pixels.length);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, width, true);
  view.setUint32(4, height, true);
  bytes.set(pixels, 8);
  return bytes.buffer;
}

describe('reading a preview frame from the camera pipeline', () => {
  it('reads the size header and keeps the RGBA pixels', () => {
    const frame = parsePreviewFrame(payload(2, 1, [1, 2, 3, 255, 4, 5, 6, 255]));
    expect(frame).not.toBeNull();
    expect(frame?.width).toBe(2);
    expect(frame?.height).toBe(1);
    expect(Array.from(frame?.rgba ?? [])).toEqual([1, 2, 3, 255, 4, 5, 6, 255]);
  });

  it('treats an empty answer as no frame yet', () => {
    expect(parsePreviewFrame(new ArrayBuffer(0))).toBeNull();
  });

  it('rejects a payload whose size and pixels disagree', () => {
    expect(parsePreviewFrame(payload(2, 2, [1, 2, 3, 255]))).toBeNull();
    expect(parsePreviewFrame(payload(0, 1, []))).toBeNull();
    expect(parsePreviewFrame(new ArrayBuffer(5))).toBeNull();
  });

  it('accepts the number array an IPC fallback may deliver', () => {
    const buffer = toArrayBuffer([2, 0, 0, 0, 1, 0, 0, 0, 9, 9, 9, 255, 8, 8, 8, 255]);
    expect(parsePreviewFrame(buffer)?.width).toBe(2);
    const raw = payload(1, 1, [1, 1, 1, 255]);
    expect(toArrayBuffer(raw)).toBe(raw);
  });
});
