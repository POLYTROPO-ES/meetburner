import type { CameraInput } from '@meetburner/core';

/** The object Tauri 2 injects into its webview; only `invoke` matters here. */
interface TauriInternals {
  invoke?: unknown;
}

const internals = (): TauriInternals | undefined =>
  (globalThis as { __TAURI_INTERNALS__?: TauriInternals }).__TAURI_INTERNALS__;

/**
 * A real webview, not merely a shell that defines the global: some browser extensions and
 * IDE previews create an empty `__TAURI_INTERNALS__`, and calling `invoke` on that throws
 * `invoke is not a function`. Every command would fail, so the section must stay hidden.
 */
export const isTauri = () => typeof internals()?.invoke === 'function';

/** Windows publishes through a DirectShow source filter (ADR-0016). */
export const isWindows = () => /windows/i.test(navigator.userAgent);
/** Linux publishes into a v4l2loopback device (ADR-0030). */
export const isLinux = () => /linux/i.test(navigator.userAgent);

/**
 * Whether this build can publish a virtual camera: Windows has a DirectShow filter and
 * Linux writes into v4l2loopback (ADR-0030). The web app and the macOS build show no
 * camera section.
 */
export const hasVirtualCamera = () => isTauri() && (isWindows() || isLinux());

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

export interface DriverStatus {
  registered: boolean;
  available: boolean;
  path: string | null;
  /** Version recorded at registration; null for installs predating the check. */
  installedVersion: number | null;
  /** Version of the driver bundled with this build. */
  expectedVersion: number | null;
}

export interface DriverUpdate {
  status: DriverStatus;
  updated: boolean;
}

export interface CameraSource {
  id: string;
  name: string;
}

/** Physical cameras the counter can be overlaid on (M7). */
export async function cameraListSources(): Promise<CameraSource[]> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<CameraSource[]>('camera_list_sources');
}

export interface V4l2Status {
  available: boolean;
  device: string | null;
}

/**
 * Whether a v4l2loopback device is loaded. Implemented on every platform but always
 * unavailable outside Linux, so callers only use it there (ADR-0030).
 */
export async function cameraV4l2Status(): Promise<V4l2Status> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<V4l2Status>('camera_v4l2_status');
}

/** Re-registers the bundled driver when the installed one is older (ADR-0020). */
export async function cameraDriverEnsure(): Promise<DriverUpdate> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<DriverUpdate>('camera_driver_ensure');
}

/** Registers "MeetBurner Camera" for the current user; no admin prompt. */
export async function cameraRegisterDriver(): Promise<DriverStatus> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<DriverStatus>('camera_register_driver');
}

/** Removes every registry key the driver created. */
export async function cameraUnregisterDriver(): Promise<DriverStatus> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<DriverStatus>('camera_unregister_driver');
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
