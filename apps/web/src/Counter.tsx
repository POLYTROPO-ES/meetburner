import { useEffect, useState } from 'react';
import { formatDuration, formatMoney, meetingSnapshot, t } from '@meetcost/core';
import { useLang, useMeeting } from './storage';

/** Current time, refreshed every animation frame while active. */
function useNow(active: boolean): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    if (!active) return;
    let id = 0;
    const tick = () => {
      setNow(Date.now());
      id = requestAnimationFrame(tick);
    };
    id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, [active]);
  return active ? now : Date.now();
}

export function Counter() {
  const [meeting, setMeeting] = useMeeting();
  const [lang] = useLang();
  const now = useNow(meeting.startAt !== null && meeting.endAt === null);
  const s = meetingSnapshot(meeting, now);
  const locale = lang === 'es' ? 'es-ES' : 'en-US';

  const status =
    meeting.startAt === null
      ? t(lang, 'notStarted')
      : s.startsInMs > 0
        ? `${t(lang, 'startsIn')} ${formatDuration(s.startsInMs)}`
        : meeting.endAt !== null
          ? t(lang, 'stopped')
          : null;

  return (
    <main className={`counter${s.running ? ' running' : ''}`}>
      <p className="label">{t(lang, 'totalCost')}</p>
      <p className="total">{formatMoney(s.totalCost, meeting.currency, locale)}</p>
      {status && <p className="status">{status}</p>}
      <dl className="stats">
        <div>
          <dt>{t(lang, 'elapsed')}</dt>
          <dd>{formatDuration(s.elapsedMs)}</dd>
        </div>
        <div>
          <dt>{t(lang, 'personTime')}</dt>
          <dd>
            {formatDuration(s.personTimeMs)} <small>({meeting.participants.length})</small>
          </dd>
        </div>
        <div>
          <dt>{t(lang, 'costPerMinute')}</dt>
          <dd>{formatMoney(s.costPerMinute, meeting.currency, locale)}</dd>
        </div>
      </dl>
      {s.running && (
        <button type="button" className="ghost" onClick={() => setMeeting({ ...meeting, endAt: Date.now() })}>
          {t(lang, 'stop')}
        </button>
      )}
    </main>
  );
}
