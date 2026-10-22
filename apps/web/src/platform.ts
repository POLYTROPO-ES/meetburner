export const isTauri = () => '__TAURI_INTERNALS__' in window;

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
  const popup = window.open('#/counter', 'meetcost-counter', 'popup,width=560,height=380');
  popup?.focus();
  return popup !== null;
}
