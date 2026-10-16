import { type CSSProperties, useEffect, useState } from "react";

import { commands, type ReserveView } from "../../lib/bindings";
import { formatTime, formatUntil } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";
import { openLink, run } from "./run";

/** A banked counter: where it should be now, and when it caps. */
export function ReserveCard({ reserve, now, onError }: { reserve: ReserveView; now: number; onError: (m: string) => void }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const reading = reserve.reading;
  const [draft, setDraft] = useState(reading ? String(reading.amount) : "");
  useEffect(() => setDraft(reading ? String(reading.amount) : ""), [reading?.amount, reading]);

  const commit = () => {
    const amount = Number.parseInt(draft, 10);
    if (Number.isFinite(amount) && amount >= 0 && amount !== reading?.amount) {
      run(commands.routineSetReading(reserve.id, amount), onError);
    }
  };

  const adjust = (delta: number) => run(commands.routineAdjustReading(reserve.id, delta), onError);
  const full = reading?.fullSinceMs != null;
  const fill = reading ? Math.min(1, reading.amount / reserve.cap) : 0;

  return (
    <article className="reserve plate" data-full={full || undefined} style={{ "--fill": `${fill * 100}%` } as CSSProperties}>
      <span className="plate-bar" aria-hidden="true" />
      <header className="reserve-head">
        <span className="reserve-name">{reserve.label}</span>
        <button type="button" className="row-action" title={t("routine.hide")} onClick={() => run(commands.routineRemove(reserve.id), onError)}>
          <Icon name="close" />
        </button>
      </header>
      <div className="reserve-body">
        <button
          type="button"
          className="step-btn reserve-step num"
          disabled={!reading || reading.amount === 0}
          title={`-${reserve.step}`}
          onClick={() => adjust(-reserve.step)}
        >
          -{reserve.step}
        </button>
        <input
          className="reserve-input num"
          inputMode="numeric"
          aria-label={reserve.label}
          placeholder="0"
          value={draft}
          onChange={(e) => setDraft(e.currentTarget.value.replace(/\D/g, "").slice(0, 6))}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        />
        <button type="button" className="step-btn reserve-step num" disabled={!reading} title={`+${reserve.step}`} onClick={() => adjust(reserve.step)}>
          +{reserve.step}
        </button>
        <span className="reserve-cap num">/ {reserve.cap}</span>
        <span className="reserve-status">
          {!reading && t("routine.notEntered")}
          {reading?.fullAtMs != null && `${t("routine.fullAt")} ${formatUntil(reading.fullAtMs, now, language, t("routine.in"))}`}
          {full && (
            <span className="reserve-full">
              {t("routine.full")}
              {reading.wasted > 0 && `, ${reading.wasted} ${t("routine.lost")}`}
            </span>
          )}
        </span>
      </div>
      <footer className="reserve-foot">
        <span className="num">
          +{reserve.regen} / {reserve.everyHours} h
        </span>
        {reading && (
          <span>
            {reading.fromGame ? t("routine.readInGame") : t("routine.enteredAt")} {formatTime(reading.recordedAtMs, language)}
          </span>
        )}
        {reading && reading.refills > 0 && (
          <span title={t("routine.chargedHint")}>
            {t("routine.charged")} <span className="num">{reading.refills}</span>
          </span>
        )}
        {!reserve.verified && <span className="unverified">{t("routine.unverified")}</span>}
        {reserve.guide && (
          <button type="button" className="text-btn" onClick={() => openLink(reserve.guide ?? "")}>
            {t("routine.guide")}
          </button>
        )}
      </footer>
    </article>
  );
}
