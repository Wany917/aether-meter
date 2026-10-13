import { useState } from "react";

import { commands, type TaskGroup } from "../../lib/bindings";
import { formatRemaining } from "../../lib/format";
import { useRoutine } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { Checkbox, Choice } from "../settings/controls";
import { useNow } from "../tracker/useNow";
import { ReserveCard } from "./ReserveCard";
import { openLink, run } from "./run";
import { TaskSection } from "./TaskSection";

/** Where corrections to the catalog go: a pull request reaches every player. */
const CATALOG_EDIT_URL = "https://github.com/Wany917/aether-meter/edit/main/catalog/routine.json";

const HOUR_CHOICES = [1, 2, 3, 4, 6, 8, 12, 24].map((h) => [String(h), `${h} h`] as const);

export function RoutineTab() {
  const routine = useRoutine();
  const { settings, t } = useSettings();
  const { language } = settings;
  const [error, setError] = useState<string | null>(null);
  // Countdowns move on the wall clock, not on performance.now().
  useNow(15_000);
  const now = Date.now();

  if (!routine) return <div className="tab-loading" />;

  const tasksOf = (group: TaskGroup) => routine.tasks.filter((task) => task.group === group);
  const { reminders } = routine;

  return (
    <div className="routine">
      <header className="routine-bar">
        <div className="routine-resets">
          <span>
            {t("routine.dailyReset")} <strong className="num">{formatRemaining(routine.nextDailyResetMs - now, language)}</strong>
          </span>
          <span>
            {t("routine.weeklyReset")} <strong className="num">{formatRemaining(routine.nextWeeklyResetMs - now, language)}</strong>
          </span>
        </div>
        <div className="routine-toggles">
          <label className="inline-check">
            <Checkbox
              checked={settings.routineOverlay}
              label={t("routine.overlay")}
              onChange={(on) => void commands.setRoutineOverlay(on)}
            />
            {t("routine.overlay")}
          </label>
          <label className="inline-check" title={t("routine.membershipHint")}>
            <Checkbox checked={routine.membership} label={t("routine.membership")} onChange={(on) => run(commands.routineSetMembership(on), setError)} />
            {t("routine.membership")}
          </label>
        </div>
      </header>

      {error && (
        <p className="settings-error" role="alert" onClick={() => setError(null)}>
          {error}
        </p>
      )}

      <section className="routine-section">
        <h2 className="routine-title">{t("routine.reserves")}</h2>
        <p className="routine-hint">{t("routine.reservesHint")}</p>
        <div className="reserve-grid">
          {routine.reserves.map((reserve) => (
            <ReserveCard key={reserve.id} reserve={reserve} now={now} onError={setError} />
          ))}
        </div>
      </section>

      <div className="task-columns">
        <TaskSection title={t("routine.daily")} group="daily" tasks={tasksOf("daily")} onError={setError} />
        <TaskSection title={t("routine.weekly")} group="weekly" tasks={tasksOf("weekly")} onError={setError} />
        <TaskSection title={t("routine.shop")} group="shop" tasks={tasksOf("shop")} onError={setError} />
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
          <Choice
            value={String(reminders.beforeResetHours)}
            options={HOUR_CHOICES}
            label={t("routine.beforeReset")}
            onChange={(v) => run(commands.routineSetReminders({ ...reminders, beforeResetHours: Number(v) }), setError)}
          />
          <span>{t("routine.beforeReset")}</span>
          <Choice
            value={String(reminders.beforeCapHours)}
            options={HOUR_CHOICES}
            label={t("routine.beforeCap")}
            onChange={(v) => run(commands.routineSetReminders({ ...reminders, beforeCapHours: Number(v) }), setError)}
          />
          <span>{t("routine.beforeCap")}</span>
        </div>
        <div className="routine-catalog">
          <span>
            {t("routine.catalog")}, {t("routine.revision")} {routine.catalogRevision} ({routine.catalogUpdated})
          </span>
          <label className="inline-check">
            <Checkbox
              checked={routine.communityUpdates}
              label={t("routine.autoUpdate")}
              onChange={(on) => run(commands.routineSetCommunityUpdates(on), setError)}
            />
            {t("routine.autoUpdate")}
          </label>
          <button type="button" className="text-btn" onClick={() => openLink(CATALOG_EDIT_URL)}>
            {t("routine.suggest")}
          </button>
          {routine.hiddenCount > 0 && (
            <button type="button" className="text-btn" onClick={() => run(commands.routineShowHidden(), setError)}>
              {t("routine.showHidden")} ({routine.hiddenCount})
            </button>
          )}
        </div>
      </footer>
    </div>
  );
}
