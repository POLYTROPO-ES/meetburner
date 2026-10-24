import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import {
  COUNTER_ELEMENTS,
  fitCost,
  formatDuration,
  formatMoney,
  formatMoneyCompact,
  resolveFont,
  resolveTheme,
  styleFor,
  t,
  type CounterElement,
  type DisplaySettings,
  type Lang,
  type Meeting,
  type MeetingSnapshot,
  type StyleOverride,
  type Theme,
} from '@meetburner/core';

interface Props {
  meeting: Meeting;
  snapshot: MeetingSnapshot;
  display: DisplaySettings;
  lang: Lang;
  now: number;
  /** Rendered inside a box (setup preview) instead of a whole window. */
  embedded?: boolean;
  /** false leaves the turn to the caller, which rotates the card box it places (ADR-0032). */
  rotateCard?: boolean;
}

/**
 * A running cost changes on every frame, so it is announced at a readable pace instead: once
 * when the counter starts and then at most once per interval (WCAG 4.1.3).
 */
const ANNOUNCE_EVERY_MS = 10_000;

/** The three CSS variables the stylesheet reads for one counter element (ADR-0029). */
function elementVars(
  element: CounterElement,
  style: StyleOverride,
  theme: Theme,
): CSSProperties & Record<string, string> {
  const elementStyle = style.elements[element];
  return {
    [`--el-${element}-font`]: resolveFont(elementStyle.font, theme),
    [`--el-${element}-size`]: String(elementStyle.scale / 100),
    // Empty = the CSS fallback, so the preset colour (including the running accent) wins.
    [`--el-${element}-color`]: elementStyle.color ?? '',
  };
}

export function CounterView({ meeting, snapshot: s, display, lang, now, embedded, rotateCard = true }: Props) {
  const theme = resolveTheme(display.theme, new Date(now));
  const locale = lang === 'es' ? 'es-ES' : 'en-US';
  const style = styleFor(display, theme);
  // Per-style customisation (ADR-0029): one font/size/colour per element, plus the background.
  const vars: CSSProperties & Record<string, string | number> = {
    '--bg': style.background.color,
    '--bg-alt': style.background.colorAlt,
    '--bg-angle':
      style.background.mode === 'solid' ? '0deg' : style.background.direction === 'horizontal' ? '90deg' : '180deg',
    '--bg-opacity': String(style.background.opacity),
    '--radius': `${style.background.radius}px`,
    '--rotation': `${rotateCard ? style.rotation : 0}deg`,
    '--text': theme.text,
    '--muted': theme.muted,
    '--accent': theme.accent,
    '--weight': theme.weight,
  };
  for (const element of COUNTER_ELEMENTS) Object.assign(vars, elementVars(element, style, theme));

  const status =
    meeting.startAt === null
      ? t(lang, 'notStarted')
      : s.startsInMs > 0
        ? `${t(lang, 'startsIn')} ${formatDuration(s.startsInMs)}`
        : meeting.endAt !== null
          ? t(lang, 'stopped')
          : null;

  // A hidden element is not rendered at all (ADR-0029): the card may end up empty.
  const shown = (element: CounterElement) => style.elements[element].visible;
  const costShown = shown('cost');
  const statsShown = shown('elapsed') || shown('personTime') || shown('costPerMinute');
  const classes = ['counter-view', s.running && 'running', embedded && 'embedded'].filter(Boolean).join(' ');

  const rootRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLParagraphElement>(null);
  const measureCompactRef = useRef<HTMLParagraphElement>(null);
  const [fit, setFit] = useState({ compact: false, scale: 1 });
  const fullCost = formatMoney(s.totalCost, meeting.currency, locale);
  const compactCost = formatMoneyCompact(s.totalCost, meeting.currency, locale);

  // Screen readers get the cost at a human pace, not sixty times a second.
  const spokenCost = `${t(lang, 'totalCost')}: ${fullCost}`;
  const spokenFinal = `${spokenCost} · ${t(lang, 'stopped')}`;
  const announcedAt = useRef(0);
  const announcedStop = useRef(false);
  const [spoken, setSpoken] = useState('');
  useEffect(() => {
    // The setup preview is decoration: only the counter window speaks.
    if (embedded) return;
    // A hidden cost is not announced either.
    if (!costShown) {
      setSpoken('');
      return;
    }
    if (s.running) {
      announcedStop.current = false;
      const at = Date.now();
      if (at - announcedAt.current >= ANNOUNCE_EVERY_MS) {
        announcedAt.current = at;
        setSpoken(spokenCost);
      }
    } else if (meeting.startAt !== null && meeting.endAt !== null && !announcedStop.current) {
      // The final total is announced once, when the meeting stops.
      announcedStop.current = true;
      announcedAt.current = 0;
      setSpoken(spokenFinal);
    }
  }, [embedded, costShown, meeting.startAt, meeting.endAt, s.running, spokenCost, spokenFinal]);
  const measure = useCallback(() => {
    if (!rootRef.current || !measureRef.current || !measureCompactRef.current) return;
    const available = rootRef.current.clientWidth * 0.9;
    const next = fitCost(available, measureRef.current.offsetWidth, measureCompactRef.current.offsetWidth, display.overflow);
    setFit((prev) => (prev.compact === next.compact && prev.scale === next.scale ? prev : next));
  }, [display.overflow]);
  useLayoutEffect(measure);
  useEffect(() => {
    const observer = new ResizeObserver(measure);
    if (rootRef.current) observer.observe(rootRef.current);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <div ref={rootRef} className={classes} style={vars} data-bg={style.background.mode}>
      <p className="sr-only" aria-live="polite">
        {spoken}
      </p>
      <div className="counter-card">
        {theme.decoration && <span className="decoration" aria-hidden="true">{theme.decoration}</span>}
        {shown('label') && <p className="label">{t(lang, 'totalCost')}</p>}
        {costShown && (
          <>
            <p className="total" title={fullCost} style={{ '--fit': fit.scale } as CSSProperties}>
              {fit.compact ? compactCost : fullCost}
            </p>
            <p ref={measureRef} className="total measure" aria-hidden="true">
              {fullCost}
            </p>
            <p ref={measureCompactRef} className="total measure" aria-hidden="true">
              {compactCost}
            </p>
          </>
        )}
        {status && shown('status') && <p className="status">{status}</p>}
        {/* One control per row: the style applies to both the heading and the value. */}
        {statsShown && (
          <dl className="stats" aria-label={t(lang, 'meetingSummary')}>
            {shown('elapsed') && (
              <div data-el="elapsed" style={elementVars('elapsed', style, theme)}>
                <dt>{t(lang, 'elapsed')}</dt>
                <dd>{formatDuration(s.elapsedMs)}</dd>
              </div>
            )}
            {shown('personTime') && (
              <div data-el="personTime" style={elementVars('personTime', style, theme)}>
                <dt>{t(lang, 'personTime')}</dt>
                <dd>
                  {formatDuration(s.personTimeMs)} <small>({meeting.participants.length})</small>
                </dd>
              </div>
            )}
            {shown('costPerMinute') && (
              <div data-el="costPerMinute" style={elementVars('costPerMinute', style, theme)}>
                <dt>{t(lang, 'costPerMinute')}</dt>
                <dd>{formatMoney(s.costPerMinute, meeting.currency, locale)}</dd>
              </div>
            )}
          </dl>
        )}
      </div>
    </div>
  );
}
