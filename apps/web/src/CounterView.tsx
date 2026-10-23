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
}

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

export function CounterView({ meeting, snapshot: s, display, lang, now, embedded }: Props) {
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

  const widget = display.layout === 'widget';
  const classes = ['counter-view', display.layout, s.running && 'running', embedded && 'embedded'].filter(Boolean).join(' ');

  const rootRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLParagraphElement>(null);
  const measureCompactRef = useRef<HTMLParagraphElement>(null);
  const [fit, setFit] = useState({ compact: false, scale: 1 });
  const fullCost = formatMoney(s.totalCost, meeting.currency, locale);
  const compactCost = formatMoneyCompact(s.totalCost, meeting.currency, locale);
  const measure = useCallback(() => {
    if (!rootRef.current || !measureRef.current || !measureCompactRef.current) return;
    // The widget must stay small so it does not cover the webcam image.
    const available = rootRef.current.clientWidth * (widget ? 0.5 : 0.9);
    const next = fitCost(available, measureRef.current.offsetWidth, measureCompactRef.current.offsetWidth, display.overflow);
    setFit((prev) => (prev.compact === next.compact && prev.scale === next.scale ? prev : next));
  }, [widget, display.overflow]);
  useLayoutEffect(measure);
  useEffect(() => {
    const observer = new ResizeObserver(measure);
    if (rootRef.current) observer.observe(rootRef.current);
    return () => observer.disconnect();
  }, [measure]);

  return (
    <div ref={rootRef} className={classes} style={vars} data-bg={style.background.mode}>
      <div className="counter-card">
        {theme.decoration && <span className="decoration" aria-hidden="true">{theme.decoration}</span>}
        {!widget && <p className="label">{t(lang, 'totalCost')}</p>}
        <p className="total" title={fullCost} style={{ '--fit': fit.scale } as CSSProperties}>
          {fit.compact ? compactCost : fullCost}
        </p>
        <p ref={measureRef} className="total measure" aria-hidden="true">
          {fullCost}
        </p>
        <p ref={measureCompactRef} className="total measure" aria-hidden="true">
          {compactCost}
        </p>
        {status && <p className="status">{status}</p>}
        {widget ? (
          <p className="time" style={elementVars('elapsed', style, theme)}>
            {formatDuration(s.elapsedMs)}
          </p>
        ) : (
          // One control per row: the style applies to both the heading and the value.
          <dl className="stats">
            <div data-el="elapsed" style={elementVars('elapsed', style, theme)}>
              <dt>{t(lang, 'elapsed')}</dt>
              <dd>{formatDuration(s.elapsedMs)}</dd>
            </div>
            <div data-el="personTime" style={elementVars('personTime', style, theme)}>
              <dt>{t(lang, 'personTime')}</dt>
              <dd>
                {formatDuration(s.personTimeMs)} <small>({meeting.participants.length})</small>
              </dd>
            </div>
            <div data-el="costPerMinute" style={elementVars('costPerMinute', style, theme)}>
              <dt>{t(lang, 'costPerMinute')}</dt>
              <dd>{formatMoney(s.costPerMinute, meeting.currency, locale)}</dd>
            </div>
          </dl>
        )}
      </div>
    </div>
  );
}
