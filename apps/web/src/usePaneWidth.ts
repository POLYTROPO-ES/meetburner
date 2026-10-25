import { useCallback, useState } from 'react';

// Renamed when the default widened, so earlier narrow widths do not stick (ADR-0034).
const PANE_KEY = 'meetburner.editorWidth';

/** Share of the workspace the editor column takes (ADR-0034). */
export const PANE_MIN = 0.3;
export const PANE_MAX = 0.6;
export const PANE_DEFAULT = 0.4;

/** Keeps a dragged width usable: never so narrow that the controls break, never more than half. */
export function clampPane(value: number): number {
  if (!Number.isFinite(value)) return PANE_DEFAULT;
  return Math.min(PANE_MAX, Math.max(PANE_MIN, value));
}

/** A stored width is a fraction; anything else falls back to the default. */
export function parsePane(raw: string | null): number {
  if (raw === null || raw.trim() === '') return PANE_DEFAULT;
  return clampPane(Number(raw));
}

/** Width of the editor column, dragged with the splitter and remembered on the device. */
export function usePaneWidth() {
  const [pane, setPane] = useState(() => parsePane(localStorage.getItem(PANE_KEY)));
  const update = useCallback((next: number) => {
    const clamped = clampPane(next);
    localStorage.setItem(PANE_KEY, String(clamped));
    setPane(clamped);
  }, []);
  return [pane, update] as const;
}
