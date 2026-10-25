import { useRef, type CSSProperties, type KeyboardEvent, type PointerEvent } from 'react';
import {
  defaultOverlayRect,
  effectiveOverlay,
  fillsFrame,
  FULL_FRAME,
  hasVideoBackground,
  moveOverlay,
  resizeOverlay,
  resolveTheme,
  styleFor,
  t,
  type CameraSettings,
  type DisplaySettings,
  type Lang,
  type Meeting,
  type MeetingSnapshot,
  type OverlayRect,
} from '@meetburner/core';
import { CounterView } from './CounterView';

interface Props {
  display: DisplaySettings;
  camera: CameraSettings;
  meeting: Meeting;
  snapshot: MeetingSnapshot;
  lang: Lang;
  now: number;
  /** Live webcam for the background, or null when there is no video (ADR-0033). */
  stream: MediaStream | null;
  /** Why the preview has no video; the counter keeps working regardless. */
  previewError: string | null;
  /** Rotation in force for the selected style, in degrees. */
  rotation: number;
  /** New card rectangle; `null` restores the built-in corner. */
  onRect: (rect: OverlayRect | null) => void;
  /** New rotation in degrees; the caller stores it with the style. */
  onRotation: (degrees: number) => void;
}

type Drag = { mode: 'move' | 'resize'; x: number; y: number; rect: OverlayRect };
type Turn = { cx: number; cy: number; start: number; rotation: number };

/** One arrow press moves or resizes the card by 1 % of the camera frame (ADR-0023). */
const KEY_STEP = 0.01;
/** One arrow press on the turn handle, and the step Shift snaps to. */
const TURN_STEP = 5;
const TURN_SNAP = 15;

const angleTo = (x: number, y: number, cx: number, cy: number) => (Math.atan2(y - cy, x - cx) * 180) / Math.PI;
/** 0..360, so a card never stores -90 degrees. */
const wrap = (value: number) => ((value % 360) + 360) % 360;

/**
 * The one place the counter is configured (ADR-0032): the card sits on the camera frame, is
 * dragged and resized on it, and turned with its handle, while the style block below edits
 * the same card. It draws the real `CounterView`, so the two can no longer disagree.
 */
export function OverlayEditor({
  display,
  camera,
  meeting,
  snapshot,
  lang,
  now,
  stream,
  previewError,
  rotation,
  onRect,
  onRotation,
}: Props) {
  const stage = useRef<HTMLDivElement>(null);
  const card = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const turn = useRef<Turn | null>(null);
  const rect = effectiveOverlay(display, camera);
  // One button for the two states: cover the frame, or go back to a card on it.
  const full = fillsFrame(rect);
  // Without video the counter always fills the frame, so there is nothing to move or resize.
  const placeable = hasVideoBackground(camera);
  const radius = styleFor(display, resolveTheme(display.theme, new Date(now))).background.radius;
  const state = !camera.video
    ? t(lang, 'overlayOnlyCounter')
    : !placeable
      ? t(lang, 'overlayPickWebcam')
      : full
        ? t(lang, 'overlayStateFull')
        : `${t(lang, 'overlayStateCard')} ${Math.round(rect.w * 100)}% × ${Math.round(rect.h * 100)}%`;

  const start = (mode: Drag['mode']) => (event: PointerEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    // Focus the card so Escape reaches its handler even when the drag began on the handle.
    card.current?.focus({ preventScroll: true });
    drag.current = { mode, x: event.clientX, y: event.clientY, rect };
  };

  const move = (event: PointerEvent<HTMLElement>) => {
    const state = drag.current;
    const box = stage.current;
    if (!state || !box || box.clientWidth === 0 || box.clientHeight === 0) return;
    const dx = (event.clientX - state.x) / box.clientWidth;
    const dy = (event.clientY - state.y) / box.clientHeight;
    onRect(state.mode === 'move' ? moveOverlay(state.rect, dx, dy) : resizeOverlay(state.rect, dx, dy));
  };

  const end = () => {
    drag.current = null;
  };

  const handlers = {
    onPointerMove: move,
    onPointerUp: end,
    onPointerCancel: end,
  };

  // Turning follows the pointer around the card centre, the way the renderer turns the layer.
  const startTurn = (event: PointerEvent<HTMLElement>) => {
    const box = card.current?.getBoundingClientRect();
    if (!box) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const cx = box.left + box.width / 2;
    const cy = box.top + box.height / 2;
    turn.current = { cx, cy, start: angleTo(event.clientX, event.clientY, cx, cy), rotation };
  };

  const turnMove = (event: PointerEvent<HTMLElement>) => {
    const state = turn.current;
    if (!state) return;
    const delta = angleTo(event.clientX, event.clientY, state.cx, state.cy) - state.start;
    const next = wrap(state.rotation + delta);
    const snapped = event.shiftKey ? (Math.round(next / TURN_SNAP) * TURN_SNAP) % 360 : Math.round(next);
    onRotation(wrap(snapped));
  };

  const turnEnd = () => {
    turn.current = null;
  };

  const turnHandlers = {
    onPointerMove: turnMove,
    onPointerUp: turnEnd,
    onPointerCancel: turnEnd,
  };

  // The same gestures from the keyboard: arrows move the card, Shift+arrows resize it.
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      // Cancels a drag in progress; the card keeps the position it last reached.
      drag.current = null;
      return;
    }
    const dx = event.key === 'ArrowRight' ? KEY_STEP : event.key === 'ArrowLeft' ? -KEY_STEP : 0;
    const dy = event.key === 'ArrowDown' ? KEY_STEP : event.key === 'ArrowUp' ? -KEY_STEP : 0;
    if (dx === 0 && dy === 0) return;
    event.preventDefault();
    onRect(event.shiftKey ? resizeOverlay(rect, dx, dy) : moveOverlay(rect, dx, dy));
  };

  // The turn handle is a slider: arrows turn the card, Home puts it upright.
  const turnKeyDown = (event: KeyboardEvent<HTMLSpanElement>) => {
    if (event.key === 'Home') {
      // The card would otherwise move: its own handler must not see this key.
      event.preventDefault();
      event.stopPropagation();
      onRotation(0);
      return;
    }
    const step =
      event.key === 'ArrowRight' || event.key === 'ArrowDown'
        ? TURN_STEP
        : event.key === 'ArrowLeft' || event.key === 'ArrowUp'
          ? -TURN_STEP
          : 0;
    if (step === 0) return;
    event.preventDefault();
    event.stopPropagation();
    onRotation(wrap(rotation + step));
  };

  return (
    <div className="overlay-editor">
      <div className="stage-fit" style={{ '--ratio': camera.width / camera.height } as CSSProperties}>
      <div
        ref={stage}
        className="overlay-stage"
        style={{
          aspectRatio: `${camera.width} / ${camera.height}`,
          width: `min(100cqw, calc(100cqh * ${camera.width / camera.height}))`,
        }}
      >
        {stream && (
          <video
            className="overlay-video"
            autoPlay
            muted
            playsInline
            ref={(element) => {
              // The stream outlives re-renders; assigning it once per stream is enough.
              if (element && element.srcObject !== stream) element.srcObject = stream;
            }}
            style={{
              transform: `scaleX(${display.mirror.sourceH ? -1 : 1}) scaleY(${display.mirror.sourceV ? -1 : 1})`,
            }}
          />
        )}
        {previewError && (
          <p className="overlay-error" role="status">
            {t(lang, 'videoPreviewFailed')}
            <span className="muted"> — {previewError}</span>
          </p>
        )}
        <div
          ref={card}
          className={placeable ? 'overlay-card' : 'overlay-card fixed'}
          tabIndex={placeable ? 0 : undefined}
          role="group"
          aria-label={t(lang, 'overlayPosition')}
          onKeyDown={placeable ? keyDown : undefined}
          style={{
            left: `${rect.x * 100}%`,
            top: `${rect.y * 100}%`,
            width: `${rect.w * 100}%`,
            height: `${rect.h * 100}%`,
            borderRadius: `${radius}px`,
            transform: `rotate(${rotation}deg)`,
          }}
          onPointerDown={placeable ? start('move') : undefined}
          {...(placeable ? handlers : {})}
        >
          <CounterView
            meeting={meeting}
            snapshot={snapshot}
            display={display}
            lang={lang}
            now={now}
            embedded
            rotateCard={false}
          />
          <span
            className="overlay-turn"
            role="slider"
            tabIndex={0}
            aria-label={t(lang, 'overlayRotate')}
            aria-valuemin={0}
            aria-valuemax={360}
            aria-valuenow={Math.round(rotation)}
            aria-valuetext={`${Math.round(rotation)}°`}
            title={t(lang, 'overlayRotate')}
            onKeyDown={turnKeyDown}
            onPointerDown={startTurn}
            {...turnHandlers}
          >
            ⟳
          </span>
          {placeable && (
            <span
              className="overlay-handle"
              aria-hidden="true"
              onPointerDown={start('resize')}
              {...handlers}
            />
          )}
        </div>
      </div>
      </div>
      <div className="stage-bar">
        {placeable && (
          <>
            <button
              type="button"
              onClick={() => onRect(full ? defaultOverlayRect(camera.width, camera.height) : FULL_FRAME)}
            >
              {full ? t(lang, 'overlayToCard') : t(lang, 'overlayFill')}
            </button>
            {display.overlay !== null && (
              <button type="button" className="ghost" onClick={() => onRect(null)}>
                {t(lang, 'overlayReset')}
              </button>
            )}
          </>
        )}
        <span className="muted">{state}</span>
        <span className="muted stage-hint">{t(lang, placeable ? 'overlayHint' : 'overlayTurnHint')}</span>
      </div>
    </div>
  );
}
