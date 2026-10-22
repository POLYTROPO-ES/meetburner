import { useState } from 'react';
import {
  applyCountry,
  COUNTRIES,
  CURRENCIES,
  encodeSummary,
  findPreset,
  FONT_IDS,
  formatMoney,
  fromLocalInputValue,
  LANGS,
  MAX_SCALE,
  meetingSnapshot,
  meetingSummary,
  MIN_SCALE,
  participantHourlyRate,
  presetSalary,
  SALARY_PRESETS,
  t,
  THEME_IDS,
  toLocalInputValue,
  totalHourlyRate,
  type CountryCode,
  type DisplaySettings,
  type FontChoice,
  type Lang,
  type Layout,
  type Participant,
  type ThemeId,
} from '@meetburner/core';
import { CounterView } from './CounterView';
import { openCounterWindow, shareUrl } from './platform';
import { useDisplay, useLang, useMeeting } from './storage';
import { useNow } from './useNow';

const DEFAULT_PRESET = 'dev-mid';
const FONT_CHOICES: FontChoice[] = ['theme', ...FONT_IDS];

export function Setup() {
  const [meeting, setMeeting] = useMeeting();
  const [lang, setLang] = useLang();
  const [display, setDisplay] = useDisplay();
  const [popupBlocked, setPopupBlocked] = useState(false);
  const [sharedLink, setSharedLink] = useState<{ url: string; copied: boolean } | null>(null);
  const running = meeting.startAt !== null && meeting.endAt === null;
  const now = useNow(running);
  const locale = lang === 'es' ? 'es-ES' : 'en-US';
  const money = (v: number) => formatMoney(v, meeting.currency, locale);
  const updateDisplay = (patch: Partial<DisplaySettings>) => setDisplay({ ...display, ...patch });

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
          annualSalary: presetSalary(DEFAULT_PRESET, meeting.country) ?? 0,
        },
      ],
    });

  const start = async () => {
    const startAt = meeting.startAt === null || meeting.endAt !== null ? Date.now() : meeting.startAt;
    setMeeting({ ...meeting, startAt, endAt: null });
    setPopupBlocked(!(await openCounterWindow(t(lang, 'appTitle'))));
  };

  const rate = totalHourlyRate(meeting);
  const summary = meetingSummary(meeting, now);

  const share = async () => {
    if (summary === null) return;
    const url = shareUrl(encodeSummary(summary));
    const copied = await navigator.clipboard?.writeText(url).then(
      () => true,
      () => false,
    );
    setSharedLink({ url, copied: copied === true });
  };

  return (
    <main className="setup">
      <header className="setup-header">
        <div className="brand">
          <img src="/favicon.svg" alt="" width={56} height={56} />
          <div>
            <h1>{t(lang, 'appTitle')}</h1>
            <p className="tagline">{t(lang, 'tagline')}</p>
          </div>
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

      <section className="intro">
        <h2>{t(lang, 'headline')}</h2>
        <p className="muted">{t(lang, 'description')}</p>
      </section>

      <section className="grid">
        <label>
          {t(lang, 'country')}
          <select value={meeting.country} onChange={(e) => setMeeting(applyCountry(meeting, e.target.value as CountryCode))}>
            {COUNTRIES.map((c) => (
              <option key={c.code} value={c.code}>
                {c.name[lang]}
              </option>
            ))}
          </select>
        </label>
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
        <label className="checkbox">
          <input
            type="checkbox"
            checked={meeting.includeEmployerCost}
            onChange={(e) => setMeeting({ ...meeting, includeEmployerCost: e.target.checked })}
          />
          {t(lang, 'includeEmployerCost')}
        </label>
        <label>
          {t(lang, 'employerCostRate')}
          <input
            type="number"
            min={0}
            max={100}
            step={0.5}
            disabled={!meeting.includeEmployerCost}
            value={Math.round(meeting.employerCostRate * 1000) / 10}
            onChange={(e) => setMeeting({ ...meeting, employerCostRate: (e.target.valueAsNumber || 0) / 100 })}
          />
        </label>
      </section>

      <section>
        <h2>{t(lang, 'participants')}</h2>
        <p className="muted">{t(lang, 'salariesAreEstimates')}</p>
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
                  updateParticipant(p.id, {
                    presetId: preset?.id ?? null,
                    annualSalary: preset?.annual[meeting.country] ?? p.annualSalary,
                  });
                }}
              >
                <option value="">{t(lang, 'customRole')}</option>
                {SALARY_PRESETS.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.label[lang]} — {money(s.annual[meeting.country])}
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
                {money(participantHourlyRate(p, meeting))}
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

      <section>
        <h2>{t(lang, 'display')}</h2>
        <div className="display-settings">
          <div className="grid">
            <label>
              {t(lang, 'style')}
              <select value={display.theme} onChange={(e) => updateDisplay({ theme: e.target.value as ThemeId })}>
                {THEME_IDS.map((id) => (
                  <option key={id} value={id}>
                    {t(lang, `theme.${id}`)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t(lang, 'layout')}
              <select value={display.layout} onChange={(e) => updateDisplay({ layout: e.target.value as Layout })}>
                <option value="fullscreen">{t(lang, 'layout.fullscreen')}</option>
                <option value="widget">{t(lang, 'layout.widget')}</option>
              </select>
            </label>
            <label>
              {t(lang, 'costFont')}
              <select value={display.costFont} onChange={(e) => updateDisplay({ costFont: e.target.value as FontChoice })}>
                {FONT_CHOICES.map((f) => (
                  <option key={f} value={f}>
                    {t(lang, `font.${f}`)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t(lang, 'timeFont')}
              <select value={display.timeFont} onChange={(e) => updateDisplay({ timeFont: e.target.value as FontChoice })}>
                {FONT_CHOICES.map((f) => (
                  <option key={f} value={f}>
                    {t(lang, `font.${f}`)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t(lang, 'costSize')} ({Math.round(display.costScale * 100)}%)
              <input
                type="range"
                min={MIN_SCALE}
                max={MAX_SCALE}
                step={0.05}
                value={display.costScale}
                onChange={(e) => updateDisplay({ costScale: e.target.valueAsNumber })}
              />
            </label>
            <label>
              {t(lang, 'timeSize')} ({Math.round(display.timeScale * 100)}%)
              <input
                type="range"
                min={MIN_SCALE}
                max={MAX_SCALE}
                step={0.05}
                value={display.timeScale}
                onChange={(e) => updateDisplay({ timeScale: e.target.valueAsNumber })}
              />
            </label>
          </div>
          <figure className="preview">
            <figcaption className="muted">{t(lang, 'preview')}</figcaption>
            <CounterView
              meeting={meeting}
              snapshot={meetingSnapshot(meeting, now)}
              display={display}
              lang={lang}
              now={now}
              embedded
            />
          </figure>
        </div>
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
          {summary !== null && (
            <button type="button" onClick={share}>
              {t(lang, 'shareCost')}
            </button>
          )}
          {meeting.startAt !== null && (
            <button type="button" className="ghost" onClick={() => setMeeting({ ...meeting, startAt: null, endAt: null })}>
              {t(lang, 'reset')}
            </button>
          )}
        </div>
        {sharedLink && (
          <p className="share-link">
            <input readOnly aria-label={t(lang, 'shareCost')} value={sharedLink.url} onFocus={(e) => e.target.select()} />
            {sharedLink.copied && <span className="muted">{t(lang, 'linkCopied')}</span>}
          </p>
        )}
        {popupBlocked && (
          <p className="muted">
            {t(lang, 'popupBlocked')} <a href="#/counter">{t(lang, 'openCounter')}</a>
          </p>
        )}
      </footer>
    </main>
  );
}
