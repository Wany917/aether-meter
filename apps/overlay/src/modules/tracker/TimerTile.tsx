import type { CSSProperties } from "react";

import type { TimerView } from "../../lib/bindings";
import { formatCountdown } from "../../lib/format";
import { GameIcon } from "../../ui/GameIcon";

const SOON_MS = 3000;

/** One buff or cooldown: its icon under a clockwise shade, time left below. */
export function TimerTile({
  timer,
  remainingMs,
  size,
  hint,
  onHide,
}: {
  timer: TimerView;
  remainingMs: number;
  size: number;
  hint: string;
  onHide: (() => void) | null;
}) {
  const elapsed = timer.totalMs > 0 ? 1 - remainingMs / timer.totalMs : 0;
  const style = { "--size": `${size}px`, "--elapsed": `${Math.max(0, Math.min(1, elapsed)) * 360}deg` } as CSSProperties;

  return (
    <button
      type="button"
      className="timer-tile"
      data-kind={timer.kind}
      data-soon={remainingMs <= SOON_MS || undefined}
      style={style}
      title={onHide ? `${timer.name}\n${hint}` : timer.name}
      disabled={!onHide}
      onClick={onHide ?? undefined}
    >
      <GameIcon candidates={timer.icons} label={timer.name} size={size} />
      <span className="timer-shade" aria-hidden="true" />
      {timer.stacks > 1 && <span className="timer-stacks num">{timer.stacks}</span>}
      <span className="timer-left num">{formatCountdown(remainingMs)}</span>
    </button>
  );
}
