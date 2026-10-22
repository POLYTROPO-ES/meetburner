import { meetingSnapshot, t } from '@meetburner/core';
import { CounterView } from './CounterView';
import { useDisplay, useLang, useMeeting } from './storage';
import { useNow } from './useNow';

export function Counter() {
  const [meeting, setMeeting] = useMeeting();
  const [lang] = useLang();
  const [display] = useDisplay();
  const now = useNow(meeting.startAt !== null && meeting.endAt === null);
  const snapshot = meetingSnapshot(meeting, now);

  return (
    <main className="counter-page">
      <CounterView meeting={meeting} snapshot={snapshot} display={display} lang={lang} now={now} />
      {snapshot.running && (
        <button type="button" className="ghost counter-stop" onClick={() => setMeeting({ ...meeting, endAt: Date.now() })}>
          {t(lang, 'stop')}
        </button>
      )}
    </main>
  );
}
