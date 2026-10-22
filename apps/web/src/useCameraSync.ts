import { useEffect, useRef } from 'react';
import type { CameraInput } from '@meetburner/core';
import { cameraStart, cameraStop, cameraUpdate, isTauri } from './platform';

/**
 * Keeps the desktop virtual camera in sync with the UI: every edit is pushed as it
 * happens, so the camera feed updates without restarting the app (ADR-0015).
 * A resolution change re-creates the shared mapping on the Rust side.
 */
export function useCameraSync(input: CameraInput | null, enabled: boolean) {
  const running = useRef(false);
  const payload = input === null ? null : JSON.stringify(input);

  useEffect(() => {
    if (!isTauri()) return;
    if (!enabled || payload === null) {
      if (running.current) {
        running.current = false;
        void cameraStop().catch((error) => console.error('meetburner: cannot stop the camera', error));
      }
      return;
    }
    const config = JSON.parse(payload) as CameraInput;
    const sync = running.current ? cameraUpdate(config) : cameraStart(config);
    running.current = true;
    void sync.catch((error) => {
      running.current = false;
      console.error('meetburner: cannot update the camera', error);
    });
  }, [enabled, payload]);
}
