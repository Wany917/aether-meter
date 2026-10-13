import type { CSSProperties } from "react";

import type { PlayerRow as Row } from "../../lib/bindings";
import { formatAmount, formatPercent } from "../../lib/format";
import { className } from "../../lib/i18n";
import { useTweened } from "../../lib/motion";
import { useSettings } from "../../lib/settings";
import { ClassMark } from "../../ui/ClassMark";
import { RankDiamond } from "../../ui/RankDiamond";
import { displayName } from "./names";

/** One player: rank, class mark, name, DPS and share, on a class-tinted bar. */
export function PlayerRow({ row, rank, fill, onOpen }: { row: Row; rank: number; fill: number; onOpen: () => void }) {
  const { settings } = useSettings();
  const { language } = settings;
  const { numberFormat, nameDisplay, showCombatPower } = settings.meter;
  const label = className(row.class, language);
  const dps = useTweened(row.dps);
  const share = useTweened(row.share);

  return (
    <button
      type="button"
      className="player-row plate"
      data-class={row.class}
      data-local={row.isLocal || undefined}
      data-idle={row.damage === 0 || undefined}
      style={{ "--fill": `${Math.max(0, Math.min(1, fill)) * 100}%` } as CSSProperties}
      onClick={onOpen}
    >
      <span className="plate-bar" aria-hidden="true" />
      <RankDiamond rank={rank} />
      <ClassMark id={row.class} label={label} />
      <span className="player-name plate-name">
        {displayName(row.name, row.class, nameDisplay, language)}
        {showCombatPower && row.combatPower > 0 && (
          <span className="player-cp num">{formatAmount(row.combatPower, "full", language)}</span>
        )}
      </span>
      <span className="player-dps num">{formatAmount(dps, numberFormat, language)}</span>
      <span className="player-total num">{formatAmount(row.damage, numberFormat, language)}</span>
      <span className="player-share num">{formatPercent(share, language)}</span>
    </button>
  );
}
