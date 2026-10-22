export const isTauri = () => '__TAURI_INTERNALS__' in window;

/** Public web URL used for share links from the desktop app (its own origin is not reachable). */
const PUBLIC_URL = import.meta.env.VITE_PUBLIC_URL ?? 'https://meetburner.pages.dev/';

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
