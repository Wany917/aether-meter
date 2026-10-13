import { useState } from "react";

import type { PlayerDetails } from "../../lib/bindings";
import { formatAmount, formatPercent } from "../../lib/format";
import { classEmblem } from "../../lib/gameAssets";
import { className } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { ClassMark } from "../../ui/ClassMark";
import { displayName } from "../meter/names";
import { SkillTable } from "./SkillTable";

/** `me` is your own line in the same fight, to compare against. */
export function PlayerPanel({ player, me }: { player: PlayerDetails; me: PlayerDetails | null }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const fmt = (n: number) => formatAmount(n, settings.meter.numberFormat, language);
  const pct = (n: number) => formatPercent(n, language);
  const [emblemFailed, setEmblemFailed] = useState(false);
  const emblem = classEmblem(player.class);
  const versusMe = me && me.actorId !== player.actorId ? player.dps - me.dps : null;

  const stats: ReadonlyArray<readonly [string, string]> = [
    [t("stat.damage"), fmt(player.damage)],
    [t("stat.share"), pct(player.share)],
    [t("stat.hits"), String(player.hits)],
    [t("stat.crit"), pct(player.critRate)],
    [t("stat.back"), pct(player.backRate)],
    [t("stat.perfect"), pct(player.perfectRate)],
    ...(player.healing > 0 ? [[t("stat.healing"), fmt(player.healing)] as const] : []),
    ...(player.combatPower > 0 ? [[t("stat.combatPower"), formatAmount(player.combatPower, "full", language)] as const] : []),
  ];

  return (
    <section className="player-panel" data-class={player.class}>
      <header className="player-head">
        <div className="player-emblem">
          {emblem && !emblemFailed ? (
            <img src={emblem} alt="" onError={() => setEmblemFailed(true)} draggable={false} />
          ) : (
            <ClassMark id={player.class} label={className(player.class, language)} size={48} />
          )}
        </div>
        <div className="player-identity">
          <span className="player-class">{className(player.class, language)}</span>
          <h2 className="player-title" data-local={player.isLocal || undefined}>
            {displayName(player.name, player.class, settings.meter.nameDisplay, language)}
          </h2>
        </div>
        <div className="player-dps-block">
          <span className="player-dps-label">{t("stat.dps")}</span>
          <span className="player-dps-value num">{fmt(player.dps)}</span>
          {versusMe !== null && (
            <span className="player-versus">
              <span className="num">{fmt(Math.abs(versusMe))}/s</span> {t(versusMe >= 0 ? "me.moreThanYou" : "me.lessThanYou")}
            </span>
          )}
        </div>
      </header>
      <dl className="player-stats">
        {stats.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd className="num">{value}</dd>
          </div>
        ))}
      </dl>
      <h3 className="band-title">{t("details.skills")}</h3>
      <div className="skill-scroll">
        <SkillTable skills={player.skills} />
      </div>
    </section>
  );
}
