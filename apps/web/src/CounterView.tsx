import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import {
  COUNTER_ELEMENTS,
  FLIP_MS,
  FLIP_SHADE,
  fitCost,
  formatDuration,
  formatMoney,
  formatMoneyCompact,
  isPlated,
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

/**
 * Created once: the counter re-renders every animation frame, so the media query is read at render
 * time instead of allocating a new one per plate per frame (ADR-0037).
 */
const REDUCED_MOTION =
  typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)')
    : null;

function prefersReducedMotion(): boolean {
  return REDUCED_MOTION?.matches ?? false;
}

/**
 * One split-flap plate (ADR-0036, ADR-0037). The plate always settles on the new glyph; when the
 * character changes, the old glyph falls away in two halves, the top turning down around the seam
 * and then the bottom turning away from it. The stylesheet owns the motion; this component only
 * decides when a flap exists and drops it once both phases are done.
 */
function Plate({ char, animate }: { char: string; animate: boolean }) {
  const previous = useRef(char);
  const [flap, setFlap] = useState<string | null>(null);
  // Read at render time: a settled plate is plain markup, with no flap left on top.
  const reduced = !animate || prefersReducedMotion();

  useLayoutEffect(() => {
    const from = previous.current;
    previous.current = char;
    if (!animate || from === char) return;
    // Re-read inside the effect, so a preference changed since the last render is honoured too.
    setFlap(prefersReducedMotion() ? null : from);
  }, [char, animate]);

  // Both phases (the bottom one delayed) are over after one FLIP_MS fall: drop the flap.
  useEffect(() => {
    if (flap === null) return;
    const timer = window.setTimeout(() => setFlap(null), FLIP_MS);
    return () => window.clearTimeout(timer);
  }, [flap]);

  return (
    <span className="plate" aria-hidden="true">
      <span className="plate-half plate-half-top">
        <span>{char}</span>
      </span>
      <span className="plate-half plate-half-bottom">
        <span>{char}</span>
      </span>
      {!reduced && flap !== null && (
        <>
          <span key={`top-${flap}`} className="plate-half plate-half-top plate-flap plate-flap-top">
            <span>{flap}</span>
          </span>
          <span key={`bottom-${flap}`} className="plate-half plate-half-bottom plate-flap plate-flap-bottom">
            <span>{flap}</span>
          </span>
        </>
      )}
    </span>
  );
}

/**
 * One split-flap plate per character; spaces advance without one (ADR-0036). Plates are keyed by
 * their position from the right, so adding a digit at the left keeps every plate to its right.
 * `animate={false}` renders settled plates for the hidden measurement rows (ADR-0037).
 */
function PlateRow({ text, animate = true }: { text: string; animate?: boolean }) {
  return (
    <>
      <span className="plates" aria-hidden="true">
        {[...text].map((char, i) =>
          char === ' ' ? (
            <span key={text.length - i} className="plate-space" />
          ) : (
            <Plate key={text.length - i} char={char} animate={animate} />
          ),
        )}
      </span>
      <span className="sr-only">{text}</span>
    </>
  );
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
    // One plate's fall and its shade, shared with the model in core (ADR-0037).
    '--flip': `${FLIP_MS}ms`,
    '--flip-shade': String(FLIP_SHADE),
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
  // Values on plates, captions plain (ADR-0036).
  const plated = (element: CounterElement) => isPlated(theme, element);
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
              {plated('cost') ? <PlateRow text={fit.compact ? compactCost : fullCost} /> : fit.compact ? compactCost : fullCost}
            </p>
            {/* The measurements must be plated too, or the fit compares a plate row with plain text;
                they never animate, they only exist to be measured (ADR-0037). */}
            <p ref={measureRef} className="total measure" aria-hidden="true">
              {plated('cost') ? <PlateRow text={fullCost} animate={false} /> : fullCost}
            </p>
            <p ref={measureCompactRef} className="total measure" aria-hidden="true">
              {plated('cost') ? <PlateRow text={compactCost} animate={false} /> : compactCost}
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
                <dd>{plated('elapsed') ? <PlateRow text={formatDuration(s.elapsedMs)} /> : formatDuration(s.elapsedMs)}</dd>
              </div>
            )}
            {shown('personTime') && (
              <div data-el="personTime" style={elementVars('personTime', style, theme)}>
                <dt>{t(lang, 'personTime')}</dt>
                <dd>
                  {plated('personTime') ? <PlateRow text={formatDuration(s.personTimeMs)} /> : formatDuration(s.personTimeMs)}{' '}
                  <small>({meeting.participants.length})</small>
                </dd>
              </div>
            )}
            {shown('costPerMinute') && (
              <div data-el="costPerMinute" style={elementVars('costPerMinute', style, theme)}>
                <dt>{t(lang, 'costPerMinute')}</dt>
                <dd>
                  {plated('costPerMinute')
                    ? <PlateRow text={formatMoney(s.costPerMinute, meeting.currency, locale)} />
                    : formatMoney(s.costPerMinute, meeting.currency, locale)}
                </dd>
              </div>
            )}
          </dl>
        )}
      </div>
    </div>
  );
}
