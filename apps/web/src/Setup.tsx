import { useEffect, useState } from 'react';
import {
  applyCountry,
  BACKGROUND_MODES,
  CAMERA_RATIOS,
  CAMERA_SIZES,
  cameraInput,
  countriesByName,
  COUNTER_ELEMENTS,
  CURRENCIES,
  defaultOverlayRect,
  encodeSummary,
  findPreset,
  FONT_IDS,
  formatMoney,
  fromLocalInputValue,
  GRADIENT_DIRECTIONS,
  LANGS,
  MAX_OPACITY,
  MAX_RADIUS,
  meetingSnapshot,
  meetingSummary,
  MIN_OPACITY,
  MIN_RADIUS,
  OVERFLOW_MODES,
  participantHourlyRate,
  presetSalary,
  resetStyle,
  resolveTheme,
  SALARY_PRESETS,
  styleFor,
  t,
  THEME_IDS,
  toLocalInputValue,
  totalHourlyRate,
  type BackgroundMode,
  type BackgroundStyle,
  type CountryCode,
  type CounterElement,
  type DisplaySettings,
  type ElementStyle,
  type FontChoice,
  type GradientDirection,
  type Lang,
  type Layout,
  type Overflow,
  type Participant,
  type StyleOverride,
  type ThemeId,
} from '@meetburner/core';
import { CounterView } from './CounterView';
import { OverlayEditor } from './OverlayEditor';
import {
  cameraDriverEnsure,
  cameraListSources,
  cameraRegisterDriver,
  cameraUnregisterDriver,
  hasVirtualCamera,
  isTauri,
  openCounterWindow,
  shareUrl,
  type CameraSource,
  type DriverStatus,
} from './platform';
import { useCamera, useDisplay, useLang, useMeeting } from './storage';
import { useCameraSync } from './useCameraSync';
import { useNow } from './useNow';

const DEFAULT_PRESET = 'dev-mid';
const FONT_CHOICES: FontChoice[] = ['theme', ...FONT_IDS];
/** `ElementStyle.scale` is a percentage (ADR-0029): 100 = the size the style ships with. */
const ELEMENT_SCALE_MIN = 50;
const ELEMENT_SCALE_MAX = 250;
const ELEMENT_SCALE_STEP = 5;
const REPO_URL = 'https://github.com/POLYTROPO-ES/meetburner';
const RELEASES_URL = 'https://github.com/POLYTROPO-ES/meetburner/releases';
const SIGNING_POLICY_URL = 'https://github.com/POLYTROPO-ES/meetburner/blob/main/docs/code-signing-policy.md';

export function Setup() {
  const [meeting, setMeeting] = useMeeting();
  const [lang, setLang] = useLang();
  const [display, setDisplay] = useDisplay();
  const [camera, setCamera] = useCamera();
  const [popupBlocked, setPopupBlocked] = useState(false);
  const [sharedLink, setSharedLink] = useState<{ url: string; copied: boolean } | null>(null);
  const running = meeting.startAt !== null && meeting.endAt === null;
  const now = useNow(running);
  const locale = lang === 'es' ? 'es-ES' : 'en-US';
  const money = (v: number) => formatMoney(v, meeting.currency, locale);
  const updateDisplay = (patch: Partial<DisplaySettings>) => setDisplay({ ...display, ...patch });
  const updateMirror = (patch: Partial<DisplaySettings['mirror']>) =>
    updateDisplay({ mirror: { ...display.mirror, ...patch } });

  // Customisation is stored per style id (ADR-0029). The controls always show what is in force,
  // and every edit stores a complete override so a touched style keeps all of its values.
  const theme = resolveTheme(display.theme, new Date(now));
  const style = styleFor(display, theme);
  const saveStyle = (next: StyleOverride) =>
    setDisplay({ ...display, styles: { ...display.styles, [display.theme]: next } });
  const updateElement = (element: CounterElement, patch: Partial<ElementStyle>) =>
    saveStyle({ ...style, elements: { ...style.elements, [element]: { ...style.elements[element], ...patch } } });
  const updateBackground = (patch: Partial<BackgroundStyle>) =>
    saveStyle({ ...style, background: { ...style.background, ...patch } });

  const [driver, setDriver] = useState<DriverStatus | null>(null);
  const [driverError, setDriverError] = useState<string | null>(null);
  const [driverUpdated, setDriverUpdated] = useState(false);
  const [sources, setSources] = useState<CameraSource[]>([]);
  // The camera is live while the app is open, as soon as the driver is registered.
  const cameraError = useCameraSync(
    cameraInput(meeting, display, lang, camera),
    hasVirtualCamera() && driver?.registered === true,
  );

  useEffect(() => {
    if (!hasVirtualCamera()) return;
    // On start, replace a driver older than the one bundled with this build (ADR-0020).
    void cameraDriverEnsure()
      .then((result) => {
        setDriver(result.status);
        setDriverUpdated(result.updated);
      })
      .catch((error) => setDriverError(String(error)));
    void cameraListSources()
      .then(setSources)
      .catch(() => setSources([]));
  }, []);

  const registerDriver = async () => {
    try {
      setDriver(await cameraRegisterDriver());
      setDriverError(null);
      setDriverUpdated(false);
    } catch (error) {
      setDriverError(String(error));
    }
  };

  const unregisterDriver = async () => {
    try {
      setDriver(await cameraUnregisterDriver());
      setDriverError(null);
      setDriverUpdated(false);
    } catch (error) {
      setDriverError(String(error));
    }
  };

  const driverMessage = driver
    ? driver.registered
      ? driver.installedVersion === null
        ? t(lang, 'cameraDriverRegistered')
        : `${t(lang, 'cameraDriverRegistered')} v${driver.installedVersion}`
      : driver.available
        ? t(lang, 'cameraDriverNotRegistered')
        : t(lang, 'cameraDriverMissing')
    : null;

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
            {countriesByName(lang).map((c) => (
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
              {t(lang, 'overflow')}
              <select value={display.overflow} onChange={(e) => updateDisplay({ overflow: e.target.value as Overflow })}>
                {OVERFLOW_MODES.map((m) => (
                  <option key={m} value={m}>
                    {t(lang, `overflow.${m}`)}
                  </option>
                ))}
              </select>
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
        <div className="style-groups">
          <div>
            <h3>{t(lang, 'styleElements')}</h3>
            <div className="style-elements">
              {COUNTER_ELEMENTS.map((element) => {
                const elementStyle = style.elements[element];
                // The colour input cannot be empty: show the preset colour until one is picked.
                const presetColor = element === 'cost' ? theme.accent : theme.muted;
                return (
                  <div key={element} className="style-element">
                    <span className="style-element-name">{t(lang, `element.${element}`)}</span>
                    <label>
                      {t(lang, 'styleFont')}
                      <select
                        value={elementStyle.font}
                        onChange={(e) => updateElement(element, { font: e.target.value as FontChoice })}
                      >
                        {FONT_CHOICES.map((f) => (
                          <option key={f} value={f}>
                            {t(lang, `font.${f}`)}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      {t(lang, 'styleSize')} ({elementStyle.scale}%)
                      <input
                        type="range"
                        min={ELEMENT_SCALE_MIN}
                        max={ELEMENT_SCALE_MAX}
                        step={ELEMENT_SCALE_STEP}
                        value={elementStyle.scale}
                        onChange={(e) => updateElement(element, { scale: e.target.valueAsNumber })}
                      />
                    </label>
                    <span className="style-color">
                      <label className="checkbox">
                        {t(lang, 'styleColor')}
                        <input
                          type="color"
                          value={elementStyle.color ?? presetColor}
                          onChange={(e) => updateElement(element, { color: e.target.value })}
                        />
                      </label>
                      <button
                        type="button"
                        className="ghost"
                        aria-label={`${t(lang, 'styleReset')}: ${t(lang, `element.${element}`)}`}
                        title={`${t(lang, 'styleReset')}: ${t(lang, `element.${element}`)}`}
                        disabled={elementStyle.color === null}
                        onClick={() => updateElement(element, { color: null })}
                      >
                        ×
                      </button>
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
          <div>
            <h3>{t(lang, 'styleBackground')}</h3>
            <div className="grid">
              <select
                className="style-mode"
                aria-label={t(lang, 'styleBackground')}
                value={style.background.mode}
                onChange={(e) => updateBackground({ mode: e.target.value as BackgroundMode })}
              >
                {BACKGROUND_MODES.map((mode) => (
                  <option key={mode} value={mode}>
                    {t(lang, `bg.mode.${mode}`)}
                  </option>
                ))}
              </select>
              <label>
                {t(lang, 'bg.color')}
                <input
                  type="color"
                  value={style.background.color}
                  onChange={(e) => updateBackground({ color: e.target.value })}
                />
              </label>
              {style.background.mode === 'gradient' && (
                <>
                  <label>
                    {t(lang, 'bg.colorAlt')}
                    <input
                      type="color"
                      value={style.background.colorAlt}
                      onChange={(e) => updateBackground({ colorAlt: e.target.value })}
                    />
                  </label>
                  <label>
                    {t(lang, 'bg.direction')}
                    <select
                      value={style.background.direction}
                      onChange={(e) => updateBackground({ direction: e.target.value as GradientDirection })}
                    >
                      {GRADIENT_DIRECTIONS.map((direction) => (
                        <option key={direction} value={direction}>
                          {t(lang, `bg.${direction}`)}
                        </option>
                      ))}
                    </select>
                  </label>
                </>
              )}
              <label>
                {t(lang, 'bg.opacity')} ({style.background.opacity}%)
                <input
                  type="range"
                  min={MIN_OPACITY}
                  max={MAX_OPACITY}
                  step={5}
                  value={style.background.opacity}
                  onChange={(e) => updateBackground({ opacity: e.target.valueAsNumber })}
                />
              </label>
              <label>
                {t(lang, 'bg.radius')} ({style.background.radius}px)
                <input
                  type="range"
                  min={MIN_RADIUS}
                  max={MAX_RADIUS}
                  step={1}
                  value={style.background.radius}
                  onChange={(e) => updateBackground({ radius: e.target.valueAsNumber })}
                />
              </label>
            </div>
          </div>
        </div>
        <div className="row">
          <button
            type="button"
            onClick={() => setDisplay(resetStyle(display))}
            disabled={display.styles[display.theme] === undefined}
          >
            {t(lang, 'styleReset')}
          </button>
        </div>
      </section>

      {hasVirtualCamera() && (
        <section>
          <h2>{t(lang, 'camera')}</h2>
          <div className="grid">
            <label>
              {t(lang, 'cameraSource')}
              <select
                value={camera.source ?? ''}
                onChange={(e) => {
                  const source = e.target.value === '' ? null : e.target.value;
                  setCamera({ ...camera, source });
                  // The card keeps the counter readable over the video (ADR-0021).
                  setDisplay({ ...display, layout: source === null ? 'fullscreen' : 'widget' });
                }}
              >
                <option value="">{t(lang, 'cameraSourceNone')}</option>
                {sources.map((source) => (
                  <option key={source.id} value={source.id}>
                    {source.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t(lang, 'cameraResolution')}
              <select
                value={`${camera.width}x${camera.height}`}
                onChange={(e) => {
                  const [width, height] = e.target.value.split('x').map(Number);
                  setCamera({ ...camera, width, height });
                }}
              >
                {CAMERA_RATIOS.map((ratio) => (
                  <optgroup key={ratio} label={ratio}>
                    {CAMERA_SIZES.filter((size) => size.ratio === ratio).map((size) => (
                      <option key={`${size.width}x${size.height}`} value={`${size.width}x${size.height}`}>
                        {size.width}×{size.height}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
            </label>
          </div>
          {camera.source !== null && <p className="muted">{t(lang, 'cameraSourceHint')}</p>}
          <p className="muted">{t(lang, 'mirrorSource')}</p>
          <div className="row">
            <label className="checkbox">
              <input
                type="checkbox"
                checked={display.mirror.sourceH}
                onChange={(e) => updateMirror({ sourceH: e.target.checked })}
              />
              {t(lang, 'mirrorHorizontal')}
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={display.mirror.sourceV}
                onChange={(e) => updateMirror({ sourceV: e.target.checked })}
              />
              {t(lang, 'mirrorVertical')}
            </label>
          </div>
          <p className="muted">{t(lang, 'mirrorOverlay')}</p>
          <div className="row">
            <label className="checkbox">
              <input
                type="checkbox"
                checked={display.mirror.overlayH}
                onChange={(e) => updateMirror({ overlayH: e.target.checked })}
              />
              {t(lang, 'mirrorHorizontal')}
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={display.mirror.overlayV}
                onChange={(e) => updateMirror({ overlayV: e.target.checked })}
              />
              {t(lang, 'mirrorVertical')}
            </label>
          </div>
          <p className="muted">{t(lang, 'cameraResolutionHint')}</p>
          {display.layout === 'widget' && (
            <>
              <p className="muted">{t(lang, 'overlayHint')}</p>
              <OverlayEditor
                rect={display.overlay ?? defaultOverlayRect(camera.width, camera.height)}
                width={camera.width}
                height={camera.height}
                meeting={meeting}
                snapshot={meetingSnapshot(meeting, now)}
                theme={display.theme}
                lang={lang}
                onChange={(rect) => updateDisplay({ overlay: rect })}
                onReset={() => updateDisplay({ overlay: null })}
              />
            </>
          )}
          <p className="muted">{t(lang, 'cameraLiveHint')}</p>
          {driver?.registered && <p className="muted">{t(lang, 'cameraAlwaysOn')}</p>}
          <div className="row">
            <button
              type="button"
              onClick={registerDriver}
              disabled={driver === null || driver.registered || !driver.available}
            >
              {t(lang, 'cameraRegister')}
            </button>
            <button
              type="button"
              onClick={unregisterDriver}
              disabled={driver === null || !driver.registered}
            >
              {t(lang, 'cameraUnregister')}
            </button>
          </div>
          {driverMessage && <p className="muted">{driverMessage}</p>}
          {driverUpdated && <p className="muted">{t(lang, 'cameraDriverUpdated')}</p>}
          {driverError && <p className="muted">{driverError}</p>}
          {cameraError && <p className="muted">{cameraError}</p>}
        </section>
      )}

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
        {!isTauri() && (
          <>
            <p className="muted">
              <a href={REPO_URL} target="_blank" rel="noreferrer">
                {t(lang, 'sourceCode')}
              </a>
              {' · '}
              <a href={RELEASES_URL} target="_blank" rel="noreferrer">
                {t(lang, 'downloads')}
              </a>
              {' · '}
              <a href={SIGNING_POLICY_URL} target="_blank" rel="noreferrer">
                {t(lang, 'codeSigningPolicy')}
              </a>
            </p>
            <p className="muted">{t(lang, 'codeSigningPolicyNote')}</p>
          </>
        )}
      </footer>
    </main>
  );
}
