import type { CameraInput } from '@meetburner/core';

export const isTauri = () => '__TAURI_INTERNALS__' in window;

/** Starts the Rust virtual camera (desktop only). */
export async function cameraStart(input: CameraInput): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('camera_start', { config: input });
}

/** Pushes a configuration change; the camera applies it on the next frame (ADR-0015). */
export async function cameraUpdate(input: CameraInput): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('camera_update', { config: input });
}

export async function cameraStop(): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('camera_stop');
}

/** Public web URL used for share links from the desktop app (its own origin is not reachable). */
const PUBLIC_URL = import.meta.env.VITE_PUBLIC_URL ?? 'https://meetburner.com/';

/** Builds the absolute `#/share?...` link for an encoded summary. */
export function shareUrl(query: string): string {
  const base = isTauri() ? PUBLIC_URL : location.origin + location.pathname;
  return `${base}#/share?${query}`;
}

/** Opens (or focuses) the counter window. Returns false if the browser blocked the popup. */
export async function openCounterWindow(title: string): Promise<boolean> {
  if (isTauri()) {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const existing = await WebviewWindow.getByLabel('counter');
    if (existing) {
      await existing.setFocus();
    } else {
      new WebviewWindow('counter', { url: 'index.html#/counter', title, width: 560, height: 380, alwaysOnTop: true });
    }
    return true;
  }
  const popup = window.open('#/counter', 'meetburner-counter', 'popup,width=560,height=380');
  popup?.focus();
  return popup !== null;
}
