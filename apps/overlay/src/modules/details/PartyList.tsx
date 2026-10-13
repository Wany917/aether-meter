import type { CSSProperties } from "react";

import type { PlayerDetails } from "../../lib/bindings";
import { formatAmount, formatPercent } from "../../lib/format";
import { className } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { ClassMark } from "../../ui/ClassMark";
import { RankDiamond } from "../../ui/RankDiamond";
import { displayName } from "../meter/names";

export function PartyList({
  players,
  selected,
  onSelect,
}: {
  players: PlayerDetails[];
  selected: number | null;
  onSelect: (actorId: number) => void;
}) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const top = players[0]?.damage ?? 0;

  return (
    <aside className="party-list">
      <h2 className="band-title">{t("details.party")}</h2>
      <ol>
        {players.map((p, i) => (
          <li key={p.actorId}>
            <button
              type="button"
              className="party-item plate"
              data-class={p.class}
              data-local={p.isLocal || undefined}
              aria-current={p.actorId === selected || undefined}
              style={{ "--fill": `${top > 0 ? (p.damage / top) * 100 : 0}%` } as CSSProperties}
              onClick={() => onSelect(p.actorId)}
            >
              <span className="plate-bar" aria-hidden="true" />
              <RankDiamond rank={i + 1} />
              <ClassMark id={p.class} label={className(p.class, language)} size={20} />
              <span className="party-name plate-name">{displayName(p.name, p.class, settings.meter.nameDisplay, language)}</span>
              <span className="party-dps num">{formatAmount(p.dps, settings.meter.numberFormat, language)}</span>
              <span className="party-share num">{formatPercent(p.share, language)}</span>
            </button>
          </li>
        ))}
      </ol>
    </aside>
  );
}
