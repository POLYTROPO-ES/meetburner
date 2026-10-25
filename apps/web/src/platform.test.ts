import { afterEach, describe, expect, it } from 'vitest';
import { hasVirtualCamera, isTauri } from './platform';

const WINDOWS = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)';
const MACOS = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)';

/** Replaces a global for one test; `navigator` is read-only in Node, so define it. */
function setGlobal(name: string, value: unknown) {
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
}

function stub(internals: unknown, userAgent = WINDOWS) {
  setGlobal('__TAURI_INTERNALS__', internals);
  setGlobal('navigator', { userAgent });
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, '__TAURI_INTERNALS__');
  Reflect.deleteProperty(globalThis, 'navigator');
});

describe('telling the desktop app from the web', () => {
  it('is the web when nothing is injected', () => {
    expect(isTauri()).toBe(false);
    expect(hasVirtualCamera()).toBe(false);
  });

  it('is the web when a shell defines an empty bridge', () => {
    // Seen in a browser shell: the global exists but has no invoke, so every command throws
    // "invoke is not a function" and the camera section must stay hidden.
    stub({});
    expect(isTauri()).toBe(false);
    expect(hasVirtualCamera()).toBe(false);
  });

  it('is the desktop app when the bridge can invoke', () => {
    stub({ invoke: () => Promise.resolve() });
    expect(isTauri()).toBe(true);
    expect(hasVirtualCamera()).toBe(true);
  });

  it('offers no virtual camera outside Windows and Linux', () => {
    stub({ invoke: () => Promise.resolve() }, MACOS);
    expect(isTauri()).toBe(true);
    expect(hasVirtualCamera()).toBe(false);
  });
});
