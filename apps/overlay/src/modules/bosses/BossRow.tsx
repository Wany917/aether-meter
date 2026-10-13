import { type CSSProperties, useState } from "react";

import { type BossView, commands } from "../../lib/bindings";
import { formatRemaining } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";
import { run } from "../routine/run";
import { clock, respawnFill, ruleText } from "./text";

const DAY_MS = 86_400_000;

/** "21:40" typed in: the last time it was 21:40, today or yesterday. */
function lastOccurrence(text: string, now: number): number | null {
  const match = /^(\d{1,2})[:h]?(\d{2})$/.exec(text.trim());
  if (!match) return null;
  const [hours, minutes] = [Number(match[1]), Number(match[2])];
  if (hours > 23 || minutes > 59) return null;
  const at = new Date(now);
  at.setHours(hours, minutes, 0, 0);
  return at.getTime() > now ? at.getTime() - DAY_MS : at.getTime();
}

function Status({ boss, now }: { boss: BossView; now: number }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const { status } = boss;
  switch (status.kind) {
    case "unknown":
      return <span className="boss-status boss-status-dim">{t("bosses.unknown")}</span>;
    case "up":
      return (
        <span className="boss-status">
          {boss.rule.kind === "schedule" ? t("bosses.live") : t("bosses.up")} <strong className="num">{clock(status.sinceMs, now, language)}</strong>
        </span>
      );
    case "respawning":
    case "scheduled":
      return (
        <span className="boss-status">
          {status.kind === "respawning" ? t("bosses.back") : t("bosses.next")} <strong className="num">{clock(status.atMs, now, language)}</strong>
          {status.atMs - now < DAY_MS && (
            <span className="boss-status-dim num">
              {" "}
              {t("routine.in")} {formatRemaining(status.atMs - now, language)}
            </span>
          )}
        </span>
      );
  }
}

export function BossRow({ boss, now, onError }: { boss: BossView; now: number; onError: (m: string) => void }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const [time, setTime] = useState("");

  const enterTime = () => {
    const at = lastOccurrence(time, now);
    if (at === null) return;
    run(commands.routineBossKilled(boss.id, at), onError);
    setTime("");
  };

  const details = [boss.zone, ruleText(boss.rule, t, language), boss.note].filter(Boolean).join(" · ");

  return (
    <li className="boss plate" data-state={boss.status.kind} style={{ "--fill": `${respawnFill(boss, now) * 100}%` } as CSSProperties}>
      <span className="plate-bar" aria-hidden="true" />
      <button
        type="button"
        className="boss-star"
        aria-pressed={boss.favorite}
        title={boss.favorite ? t("bosses.unfollow") : t("bosses.follow")}
        onClick={() => run(commands.routineBossFavorite(boss.id, !boss.favorite), onError)}
      >
        <Icon name="star" fill={boss.favorite ? "currentColor" : "none"} />
      </button>
      <div className="task-text">
        <span className="task-name">{boss.label}</span>
        <span className="task-note">
          {details}
          {boss.lastKillMs != null && ` · ${t("bosses.lastKill")} ${clock(boss.lastKillMs, now, language)}`}
          {!boss.verified && <span className="unverified"> {t("routine.unverified")}</span>}
        </span>
      </div>
      <Status boss={boss} now={now} />
      <span className="boss-actions">
        <button type="button" className="boss-kill" title={t("bosses.killedNow")} onClick={() => run(commands.routineBossKilled(boss.id, null), onError)}>
          {t("bosses.killed")}
        </button>
        <input
          className="boss-time num"
          inputMode="numeric"
          placeholder="hh:mm"
          aria-label={t("bosses.killedAt")}
          title={t("bosses.killedAt")}
          value={time}
          maxLength={5}
          onChange={(e) => setTime(e.currentTarget.value.replace(/[^\dh:]/g, ""))}
          onKeyDown={(e) => e.key === "Enter" && enterTime()}
          onBlur={enterTime}
        />
      </span>
      <button
        type="button"
        className="row-action"
        title={t("bosses.forget")}
        disabled={boss.lastKillMs == null}
        onClick={() => run(commands.routineBossForget(boss.id), onError)}
      >
        <Icon name="close" />
      </button>
    </li>
  );
}
