import { useRef, type PointerEvent } from 'react';
import {
  formatDuration,
  formatMoney,
  moveOverlay,
  resizeOverlay,
  resolveTheme,
  t,
  type Lang,
  type Meeting,
  type MeetingSnapshot,
  type OverlayRect,
  type ThemeId,
} from '@meetburner/core';

interface Props {
  /** Card rectangle as fractions of the camera frame. */
  rect: OverlayRect;
  /** Camera output size, used for the aspect ratio of the stage. */
  width: number;
  height: number;
  meeting: Meeting;
  snapshot: MeetingSnapshot;
  theme: ThemeId;
  lang: Lang;
  onChange: (rect: OverlayRect) => void;
  onReset: () => void;
}

type Drag = { mode: 'move' | 'resize'; x: number; y: number; rect: OverlayRect };

/**
 * Drag-and-drop placement of the counter card on the camera frame (ADR-0023): drag the card
 * to move it, drag its bottom-right corner to resize. The real renderer computes the font
 * sizes from the card, so this is a gesture surface, not a pixel-perfect preview.
 */
export function OverlayEditor({ rect, width, height, meeting, snapshot, theme, lang, onChange, onReset }: Props) {
  const stage = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const palette = resolveTheme(theme, new Date());
  const locale = lang === 'es' ? 'es-ES' : 'en-US';

  const start = (mode: Drag['mode']) => (event: PointerEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { mode, x: event.clientX, y: event.clientY, rect };
  };

  const move = (event: PointerEvent<HTMLElement>) => {
    const state = drag.current;
    const box = stage.current;
    if (!state || !box || box.clientWidth === 0 || box.clientHeight === 0) return;
    const dx = (event.clientX - state.x) / box.clientWidth;
    const dy = (event.clientY - state.y) / box.clientHeight;
    onChange(state.mode === 'move' ? moveOverlay(state.rect, dx, dy) : resizeOverlay(state.rect, dx, dy));
  };

  const end = () => {
    drag.current = null;
  };

  const handlers = {
    onPointerMove: move,
    onPointerUp: end,
    onPointerCancel: end,
  };

  return (
    <div className="overlay-editor">
      <div
        ref={stage}
        className="overlay-stage"
        style={{ aspectRatio: `${width} / ${height}` }}
        aria-label={t(lang, 'overlayPosition')}
        role="group"
      >
        <div
          className="overlay-card"
          style={{
            left: `${rect.x * 100}%`,
            top: `${rect.y * 100}%`,
            width: `${rect.w * 100}%`,
            height: `${rect.h * 100}%`,
            background: palette.background,
            color: palette.text,
            borderRadius: `${Math.max(4, Math.min(24, palette.radius))}px`,
          }}
          onPointerDown={start('move')}
          {...handlers}
        >
          <span className="overlay-cost">
            {formatMoney(snapshot.totalCost, meeting.currency, locale)}
          </span>
          <span className="overlay-time">{formatDuration(snapshot.elapsedMs)}</span>
          <span
            className="overlay-handle"
            aria-hidden="true"
            onPointerDown={start('resize')}
            {...handlers}
          />
        </div>
      </div>
      <div className="row">
        <button type="button" onClick={onReset}>
          {t(lang, 'overlayReset')}
        </button>
      </div>
    </div>
  );
}
