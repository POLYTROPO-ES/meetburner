import { useCallback, useEffect, useState } from 'react';
import {
  createMeeting,
  detectLang,
  LANGS,
  normalizeCamera,
  normalizeDisplay,
  type CameraSettings,
  type DisplaySettings,
  type Lang,
  type Meeting,
} from '@meetburner/core';

const MEETING_KEY = 'meetburner.meeting';
const LANG_KEY = 'meetburner.lang';
const DISPLAY_KEY = 'meetburner.display';
const CAMERA_KEY = 'meetburner.camera';

function parseMeeting(raw: string | null): Meeting {
  try {
    const parsed = raw ? JSON.parse(raw) : null;
    if (parsed && typeof parsed === 'object' && Array.isArray(parsed.participants)) {
      return { ...createMeeting(), ...parsed };
    }
  } catch {
    // Corrupt storage: fall back to defaults.
  }
  return createMeeting();
}

function parseLang(raw: string | null): Lang {
  return LANGS.includes(raw as Lang) ? (raw as Lang) : detectLang(navigator.language);
}

function parseDisplay(raw: string | null): DisplaySettings {
  try {
    return normalizeDisplay(raw ? JSON.parse(raw) : null);
  } catch {
    return normalizeDisplay(null);
  }
}

function parseCamera(raw: string | null): CameraSettings {
  try {
    return normalizeCamera(raw ? JSON.parse(raw) : null);
  } catch {
    return normalizeCamera(null);
  }
}

/** State persisted in localStorage and kept in sync across windows. */
function useStored<T>(key: string, parse: (raw: string | null) => T, serialize: (v: T) => string) {
  const [value, setValue] = useState(() => parse(localStorage.getItem(key)));

  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === key) setValue(parse(e.newValue));
    };
    addEventListener('storage', onStorage);
    return () => removeEventListener('storage', onStorage);
  }, [key, parse]);

  const update = useCallback(
    (next: T) => {
      localStorage.setItem(key, serialize(next));
      setValue(next);
    },
    [key, serialize],
  );

  return [value, update] as const;
}

export const useMeeting = () => useStored(MEETING_KEY, parseMeeting, JSON.stringify);
export const useDisplay = () => useStored(DISPLAY_KEY, parseDisplay, JSON.stringify);
export const useCamera = () => useStored(CAMERA_KEY, parseCamera, JSON.stringify);

/** The UI language; the document language follows it so screen readers pick the right voice. */
export function useLang() {
  const [lang, setLang] = useStored(LANG_KEY, parseLang, String);
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  return [lang, setLang] as const;
}
