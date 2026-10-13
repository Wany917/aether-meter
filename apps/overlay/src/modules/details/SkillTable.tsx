import type { CSSProperties } from "react";

import type { SkillRow } from "../../lib/bindings";
import { formatAmount, formatPercent } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { GameIcon } from "../../ui/GameIcon";

/** One plate per skill, its bar sized against the player's best skill. */
export function SkillTable({ skills }: { skills: SkillRow[] }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const fmt = (n: number) => formatAmount(n, settings.meter.numberFormat, language);
  const pct = (n: number) => formatPercent(n, language, 0);
  const top = skills[0]?.damage ?? 0;

  if (skills.length === 0) return <p className="empty-note">{t("details.noSkills")}</p>;

  return (
    <div className="skill-table" role="table">
      <div className="skill-head" role="row">
        <span role="columnheader">{t("stat.skill")}</span>
        <span role="columnheader">{t("stat.damage")}</span>
        <span role="columnheader">{t("stat.share")}</span>
        <span role="columnheader">{t("stat.hits")}</span>
        <span role="columnheader">{t("stat.crit")}</span>
        <span role="columnheader">{t("stat.back")}</span>
        <span role="columnheader">{t("stat.perfect")}</span>
        <span role="columnheader">{t("stat.avg")}</span>
        <span role="columnheader">{t("stat.max")}</span>
      </div>
      {skills.map((s) => (
        <div
          key={`${s.code}:${s.isDot}`}
          className="skill-row plate"
          role="row"
          style={{ "--fill": `${top > 0 ? (s.damage / top) * 100 : 0}%` } as CSSProperties}
        >
          <span className="plate-bar" aria-hidden="true" />
          <span className="skill-cell-name" role="cell">
            <GameIcon candidates={s.icons} label={s.name} size={21} />
            <span className="skill-name">{s.name}</span>
          </span>
          <span className="num skill-damage" role="cell">
            {fmt(s.damage)}
          </span>
          <span className="num" role="cell">
            {formatPercent(s.share, language)}
          </span>
          <span className="num" role="cell">
            {s.hits}
          </span>
          <span className="num" role="cell">
            {s.isDot ? "" : pct(s.critRate)}
          </span>
          <span className="num" role="cell">
            {s.isDot ? "" : pct(s.backRate)}
          </span>
          <span className="num" role="cell">
            {s.isDot ? "" : pct(s.perfectRate)}
          </span>
          <span className="num" role="cell">
            {fmt(s.avgHit)}
          </span>
          <span className="num" role="cell">
            {fmt(s.maxHit)}
          </span>
        </div>
      ))}
    </div>
  );
}
