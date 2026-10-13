import type { FightView } from "../../lib/bindings";
import { formatAmount, formatDay, formatDuration, formatTime } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";

export function FightHeader({ fight, onBackToLive }: { fight: FightView; onBackToLive: (() => void) | null }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const fmt = (n: number) => formatAmount(n, settings.meter.numberFormat, language);
  const live = fight.source.kind === "live";
  const partyDps = fight.durationMs > 0 ? fight.totalDamage / Math.max(1, fight.durationMs / 1000) : 0;

  return (
    <header className="fight-header">
      <div className="fight-identity">
        <div className="fight-kicker">
          {live ? (
            <span className="fight-live">{t("details.live")}</span>
          ) : (
            <span>
              {fight.startedAtMs > 0 && `${formatDay(fight.startedAtMs, language)}, ${formatTime(fight.startedAtMs, language)}`}
            </span>
          )}
          {fight.dungeon && <span className="fight-dungeon">{fight.dungeon}</span>}
        </div>
        <h1 className="fight-title">{fight.targetName || t("meter.noTarget")}</h1>
      </div>
      <dl className="fight-stats">
        <div>
          <dt>{t("stat.duration")}</dt>
          <dd className="num">{formatDuration(fight.durationMs)}</dd>
        </div>
        <div>
          <dt>{t("stat.damage")}</dt>
          <dd className="num">{fmt(fight.totalDamage)}</dd>
        </div>
        <div>
          <dt>{t("meter.party")}</dt>
          <dd className="num gold">{fmt(partyDps)}</dd>
        </div>
        {fight.maxHp > 0 && (
          <div>
            <dt>{t("stat.maxHp")}</dt>
            <dd className="num">{fmt(fight.maxHp)}</dd>
          </div>
        )}
      </dl>
      {onBackToLive && (
        <button type="button" className="btn btn-slate" onClick={onBackToLive}>
          <Icon name="back" width={13} height={13} />
          {t("details.backToLive")}
        </button>
      )}
    </header>
  );
}
