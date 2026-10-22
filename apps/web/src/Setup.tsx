import { useState } from 'react';
import {
  CURRENCIES,
  findPreset,
  formatMoney,
  fromLocalInputValue,
  hourlyRate,
  LANGS,
  SALARY_PRESETS_ES,
  t,
  toLocalInputValue,
  totalHourlyRate,
  type Lang,
  type Participant,
} from '@meetcost/core';
import { openCounterWindow } from './platform';
import { useLang, useMeeting } from './storage';

const DEFAULT_PRESET = 'dev-mid';

export function Setup() {
  const [meeting, setMeeting] = useMeeting();
  const [lang, setLang] = useLang();
  const [popupBlocked, setPopupBlocked] = useState(false);
  const locale = lang === 'es' ? 'es-ES' : 'en-US';
  const money = (v: number) => formatMoney(v, meeting.currency, locale);

  const updateParticipant = (id: string, patch: Partial<Participant>) =>
    setMeeting({
      ...meeting,
      participants: meeting.participants.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    });

  const addParticipant = () =>
    setMeeting({
      ...meeting,
      participants: [
        ...meeting.participants,
        {
          id: crypto.randomUUID(),
          name: '',
          presetId: DEFAULT_PRESET,
          annualSalary: findPreset(DEFAULT_PRESET)!.annual,
        },
      ],
    });

  const start = async () => {
    const now = Date.now();
    const startAt = meeting.startAt === null || meeting.endAt !== null ? now : meeting.startAt;
    setMeeting({ ...meeting, startAt, endAt: null });
    setPopupBlocked(!(await openCounterWindow(t(lang, 'appTitle'))));
  };

  const running = meeting.startAt !== null && meeting.endAt === null;
  const rate = totalHourlyRate(meeting);

  return (
    <main className="setup">
      <header className="setup-header">
        <div>
          <h1>{t(lang, 'appTitle')}</h1>
          <p className="muted">{t(lang, 'tagline')}</p>
        </div>
        <label>
          {t(lang, 'language')}
          <select value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
            {LANGS.map((l) => (
              <option key={l} value={l}>
                {l.toUpperCase()}
              </option>
            ))}
          </select>
        </label>
      </header>

      <section className="grid">
        <label>
          {t(lang, 'currency')}
          <select value={meeting.currency} onChange={(e) => setMeeting({ ...meeting, currency: e.target.value })}>
            {CURRENCIES.map((c) => (
              <option key={c}>{c}</option>
            ))}
          </select>
        </label>
        <label>
          {t(lang, 'hoursPerYear')}
          <input
            type="number"
            min={1}
            value={meeting.hoursPerYear}
            onChange={(e) => setMeeting({ ...meeting, hoursPerYear: e.target.valueAsNumber || 0 })}
          />
        </label>
        <label>
          {t(lang, 'startTime')}
          <span className="row">
            <input
              type="datetime-local"
              value={meeting.startAt === null ? '' : toLocalInputValue(meeting.startAt)}
              onChange={(e) => setMeeting({ ...meeting, startAt: fromLocalInputValue(e.target.value), endAt: null })}
            />
            <button type="button" onClick={() => setMeeting({ ...meeting, startAt: Date.now(), endAt: null })}>
              {t(lang, 'now')}
            </button>
          </span>
        </label>
      </section>

      <section>
        <h2>{t(lang, 'participants')}</h2>
        <ul className="participants">
          {meeting.participants.map((p) => (
            <li key={p.id} className="participant">
              <input
                aria-label={t(lang, 'name')}
                placeholder={t(lang, 'name')}
                value={p.name}
                onChange={(e) => updateParticipant(p.id, { name: e.target.value })}
              />
              <select
                aria-label={t(lang, 'role')}
                value={p.presetId ?? ''}
                onChange={(e) => {
                  const preset = findPreset(e.target.value || null);
                  updateParticipant(p.id, { presetId: preset?.id ?? null, annualSalary: preset?.annual ?? p.annualSalary });
                }}
              >
                <option value="">{t(lang, 'customRole')}</option>
                {SALARY_PRESETS_ES.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.label[lang]} — {formatMoney(s.annual, 'EUR', locale)}
                  </option>
                ))}
              </select>
              <input
                type="number"
                min={0}
                step={1000}
                aria-label={t(lang, 'annualSalary')}
                value={p.annualSalary}
                onChange={(e) => updateParticipant(p.id, { presetId: null, annualSalary: e.target.valueAsNumber || 0 })}
              />
              <span className="rate">
                {money(hourlyRate(p.annualSalary, meeting.hoursPerYear))}
                {t(lang, 'perHour')}
              </span>
              <button
                type="button"
                className="ghost"
                onClick={() => setMeeting({ ...meeting, participants: meeting.participants.filter((x) => x.id !== p.id) })}
              >
                {t(lang, 'remove')}
              </button>
            </li>
          ))}
        </ul>
        <button type="button" onClick={addParticipant}>
          + {t(lang, 'addParticipant')}
        </button>
      </section>

      <footer className="setup-footer">
        <p>
          {meeting.participants.length}{' '}
          {t(lang, meeting.participants.length === 1 ? 'personSingular' : 'personPlural')} · {money(rate)}
          {t(lang, 'perHour')} · {money(rate / 60)} / min
        </p>
        {meeting.participants.length === 0 && <p className="muted">{t(lang, 'noParticipants')}</p>}
        <div className="row">
          <button type="button" className="primary" disabled={meeting.participants.length === 0} onClick={start}>
            {running ? t(lang, 'openCounter') : t(lang, 'start')}
          </button>
          {running && (
            <button type="button" onClick={() => setMeeting({ ...meeting, endAt: Date.now() })}>
              {t(lang, 'stop')}
            </button>
          )}
          {meeting.startAt !== null && (
            <button type="button" className="ghost" onClick={() => setMeeting({ ...meeting, startAt: null, endAt: null })}>
              {t(lang, 'reset')}
            </button>
          )}
        </div>
        {popupBlocked && (
          <p className="muted">
            {t(lang, 'popupBlocked')} <a href="#/counter">{t(lang, 'openCounter')}</a>
          </p>
        )}
      </footer>
    </main>
  );
}
