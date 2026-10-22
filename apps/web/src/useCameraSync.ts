import { useEffect, useRef, useState } from 'react';
import type { CameraInput } from '@meetburner/core';
import { cameraStart, cameraStop, cameraUpdate, isTauri } from './platform';

/**
 * Keeps the virtual camera running while the app is open (ADR-0018): it starts as soon
 * as the driver is registered, pushes every edit on the next frame and stops when the
 * driver is cleaned. Failures are returned so the UI can show them; the app keeps running.
 */
export function useCameraSync(input: CameraInput, active: boolean): string | null {
  const running = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const payload = JSON.stringify(input);

  useEffect(() => {
    if (!isTauri()) return;
    if (!active) {
      if (running.current) {
        running.current = false;
        void cameraStop().catch(() => undefined);
      }
      setError(null);
      return;
    }
    const config = JSON.parse(payload) as CameraInput;
    const sync = running.current ? cameraUpdate(config) : cameraStart(config);
    running.current = true;
    void sync.then(
      () => setError(null),
      (reason) => {
        running.current = false;
        setError(String(reason));
      },
    );
  }, [active, payload]);

  return error;
}
