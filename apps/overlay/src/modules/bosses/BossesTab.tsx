import { useState } from "react";

import { type BossView, commands } from "../../lib/bindings";
import { useRoutine } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { openLink, run } from "../routine/run";
import { Checkbox, Choice } from "../settings/controls";
import { useNow } from "../tracker/useNow";
import { BossRow } from "./BossRow";
import { bySoonest } from "./text";

const CATALOG_EDIT_URL = "https://github.com/Wany917/aether-meter/edit/main/catalog/routine.json";
const LEAD_CHOICES = [1, 3, 5, 10, 15, 30].map((m) => [String(m), `${m} min`] as const);

function BossList({ title, hint, bosses, now, onError }: { title: string; hint?: string; bosses: BossView[]; now: number; onError: (m: string) => void }) {
  if (bosses.length === 0) return null;
  return (
    <section className="task-section">
      <h2 className="routine-title">
        {title}
        <span className="routine-count num">{bosses.length}</span>
      </h2>
      {hint && <p className="routine-hint">{hint}</p>}
      <ol className="task-list">
        {bosses.map((boss) => (
          <BossRow key={boss.id} boss={boss} now={now} onError={onError} />
        ))}
      </ol>
    </section>
  );
}

/** Every boss that comes back, the ones the player follows on the overlay. */
export function BossesTab() {
  const routine = useRoutine();
  const { settings, t } = useSettings();
  const [followedOnly, setFollowedOnly] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useNow(15_000);
  const now = Date.now();

  if (!routine) return <div className="tab-loading" />;

  const followed = routine.bosses.filter((b) => b.favorite).length;
  const shown = routine.bosses.filter((b) => !followedOnly || b.favorite).sort(bySoonest);
  const { reminders } = routine;

  return (
    <div className="routine bosses">
      <header className="routine-bar">
        <div className="routine-resets">
          <span>
            {t("bosses.followed")} <strong className="num">{followed}</strong>
          </span>
        </div>
        <div className="routine-toggles">
          <label className="inline-check">
            <Checkbox checked={followedOnly} label={t("bosses.followedOnly")} onChange={setFollowedOnly} />
            {t("bosses.followedOnly")}
          </label>
          <label className="inline-check">
            <Checkbox checked={settings.routineOverlay} label={t("routine.overlay")} onChange={(on) => void commands.setRoutineOverlay(on)} />
            {t("routine.overlay")}
          </label>
        </div>
      </header>

      {error && (
        <p className="settings-error" role="alert" onClick={() => setError(null)}>
          {error}
        </p>
      )}

      <p className="routine-hint bosses-hint">{t("bosses.hint")}</p>
      {followedOnly && followed === 0 && <p className="routine-hint">{t("bosses.none")}</p>}

      <div className="task-columns boss-columns">
        <BossList title={t("bosses.field")} bosses={shown.filter((b) => b.rule.kind === "afterKill")} now={now} onError={setError} />
        <BossList
          title={t("bosses.scheduled")}
          hint={t("bosses.scheduledHint")}
          bosses={shown.filter((b) => b.rule.kind === "schedule")}
          now={now}
          onError={setError}
        />
      </div>

      <footer className="routine-foot">
        <div className="routine-reminders">
          <label className="inline-check">
            <Checkbox
              checked={reminders.enabled}
              label={t("routine.reminders")}
              onChange={(enabled) => run(commands.routineSetReminders({ ...reminders, enabled }), setError)}
            />
            {t("routine.reminders")}
          </label>
          <span>{t("bosses.remind")}</span>
          <Choice
            value={String(routine.bossLeadMinutes)}
            options={LEAD_CHOICES}
            label={t("bosses.lead")}
            onChange={(v) => run(commands.routineSetBossLead(Number(v)), setError)}
          />
          <span>{t("bosses.lead")}</span>
        </div>
        <div className="routine-catalog">
          <span>
            {t("routine.catalog")}, {t("routine.revision")} {routine.catalogRevision}
          </span>
          <button type="button" className="text-btn" onClick={() => openLink(CATALOG_EDIT_URL)}>
            {t("routine.suggest")}
          </button>
        </div>
      </footer>
    </div>
  );
}
