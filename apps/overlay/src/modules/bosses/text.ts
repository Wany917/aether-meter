import type { BossRule, BossView } from "../../lib/bindings";
import { formatRemaining, formatSlot, formatTime } from "../../lib/format";
import type { Translate } from "../../lib/i18n";

const HOUR_MS = 3_600_000;
const DAY_MS = 24 * HOUR_MS;

/** Up first, then whoever comes back soonest, then the ones never seen killed. */
export function bySoonest(a: BossView, b: BossView): number {
  return rank(a) - rank(b) || moment(a) - moment(b);
}

function rank(boss: BossView): number {
  if (boss.status.kind === "up") return 0;
  return boss.status.kind === "unknown" ? 2 : 1;
}

function moment(boss: BossView): number {
  switch (boss.status.kind) {
    case "up":
      return boss.status.sinceMs;
    case "respawning":
    case "scheduled":
      return boss.status.atMs;
    case "unknown":
      return 0;
  }
}

/** "21:40" within a day, "ven. 21:40" beyond. */
export function clock(epochMs: number, now: number, language: string): string {
  return Math.abs(epochMs - now) < DAY_MS ? formatTime(epochMs, language) : formatSlot(epochMs, language);
}

/** "12 h après la mort", "ven. 21:00, dim. 21:00", "toutes les 3 h". */
export function ruleText(rule: BossRule, t: Translate, language: string): string {
  if (rule.kind === "afterKill") {
    const { minutes } = rule;
    const span = minutes % 60 === 0 ? `${minutes / 60} h` : `${minutes} min`;
    return `${span} ${t("bosses.afterKill")}`;
  }
  const slots = rule.nextSlotsMs;
  if (rule.daily && slots.length > 3) return `${t("bosses.every")} ${Math.round(24 / slots.length)} h`;
  if (rule.daily) return `${t("bosses.daily")} ${slots.map((ms) => formatTime(ms, language)).join(", ")}`;
  return slots.map((ms) => formatSlot(ms, language)).join(", ");
}

/** The overlay's one-word status: "Disponible", "dans 12 min", "21:40". */
export function shortStatus(boss: BossView, now: number, t: Translate, language: string): string {
  switch (boss.status.kind) {
    case "up":
      return t("bosses.nowUp");
    case "respawning":
    case "scheduled": {
      const at = boss.status.atMs;
      return at - now <= HOUR_MS ? `${t("routine.in")} ${formatRemaining(at - now, language)}` : clock(at, now, language);
    }
    case "unknown":
      return "";
  }
}

/** How far a field boss is through its respawn, for the plate's bar. */
export function respawnFill(boss: BossView, now: number): number {
  if (boss.status.kind === "up") return 1;
  if (boss.status.kind !== "respawning" || boss.rule.kind !== "afterKill") return 0;
  const total = boss.rule.minutes * 60_000;
  return Math.min(1, Math.max(0, 1 - (boss.status.atMs - now) / total));
}
