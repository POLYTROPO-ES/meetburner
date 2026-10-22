import type { CSSProperties } from 'react';
import {
  formatDuration,
  formatMoney,
  resolveFont,
  resolveTheme,
  t,
  type DisplaySettings,
  type Lang,
  type Meeting,
  type MeetingSnapshot,
} from '@meetcost/core';

interface Props {
  meeting: Meeting;
  snapshot: MeetingSnapshot;
  display: DisplaySettings;
  lang: Lang;
  now: number;
  /** Rendered inside a box (setup preview) instead of a whole window. */
  embedded?: boolean;
}

export function CounterView({ meeting, snapshot: s, display, lang, now, embedded }: Props) {
  const theme = resolveTheme(display.theme, new Date(now));
  const locale = lang === 'es' ? 'es-ES' : 'en-US';
  const style = {
    '--bg': theme.background,
    '--bg-alt': theme.backgroundAlt,
    '--text': theme.text,
    '--muted': theme.muted,
    '--accent': theme.accent,
    '--radius': `${theme.radius}px`,
    '--weight': theme.weight,
    '--cost-font': resolveFont(display.costFont, theme),
    '--time-font': resolveFont(display.timeFont, theme),
    '--cost-scale': display.costScale,
    '--time-scale': display.timeScale,
  } as CSSProperties;

  const status =
    meeting.startAt === null
      ? t(lang, 'notStarted')
      : s.startsInMs > 0
        ? `${t(lang, 'startsIn')} ${formatDuration(s.startsInMs)}`
        : meeting.endAt !== null
          ? t(lang, 'stopped')
          : null;

  const widget = display.layout === 'widget';
  const classes = ['counter-view', display.layout, s.running && 'running', embedded && 'embedded'].filter(Boolean).join(' ');

  return (
    <div className={classes} style={style}>
      <div className="counter-card">
        {theme.decoration && <span className="decoration" aria-hidden="true">{theme.decoration}</span>}
        {!widget && <p className="label">{t(lang, 'totalCost')}</p>}
        <p className="total">{formatMoney(s.totalCost, meeting.currency, locale)}</p>
        {status && <p className="status">{status}</p>}
        {widget ? (
          <p className="time">{formatDuration(s.elapsedMs)}</p>
        ) : (
          <dl className="stats">
            <div>
              <dt>{t(lang, 'elapsed')}</dt>
              <dd className="time">{formatDuration(s.elapsedMs)}</dd>
            </div>
            <div>
              <dt>{t(lang, 'personTime')}</dt>
              <dd className="time">
                {formatDuration(s.personTimeMs)} <small>({meeting.participants.length})</small>
              </dd>
            </div>
            <div>
              <dt>{t(lang, 'costPerMinute')}</dt>
              <dd>{formatMoney(s.costPerMinute, meeting.currency, locale)}</dd>
            </div>
          </dl>
        )}
      </div>
    </div>
  );
}
