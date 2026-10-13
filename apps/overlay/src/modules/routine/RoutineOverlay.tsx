import { type CSSProperties, useRef } from "react";

import { commands, type ReserveView, type TaskView } from "../../lib/bindings";
import { formatRemaining } from "../../lib/format";
import { usePets, useRoutine } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { useFitWindowHeight } from "../../lib/windowFit";
import { Icon } from "../../ui/Icon";
import { bySoonest, shortStatus } from "../bosses/text";
import { petName } from "../pets/text";
import { useNow } from "../tracker/useNow";
import { run } from "./run";

const MAX_WEEKLY = 5;
const MAX_BOSSES = 6;
const DAY_MS = 86_400_000;

/** Full reserves first, then the ones that will cap soonest. */
function urgency(reserve: ReserveView): number {
  const reading = reserve.reading;
  if (!reading) return Number.POSITIVE_INFINITY;
  if (reading.fullSinceMs != null) return Number.NEGATIVE_INFINITY;
  return reading.fullAtMs ?? Number.POSITIVE_INFINITY;
}

function OverlayTask({ task, locked }: { task: TaskView; locked: boolean }) {
  return (
    <li className="ro-task plate" style={{ "--fill": `${(task.progress / task.target) * 100}%` } as CSSProperties}>
      <span className="plate-bar" aria-hidden="true" />
      <button
        type="button"
        role="checkbox"
        aria-checked={false}
        aria-label={task.label}
        className="check"
        disabled={locked}
        onClick={() => run(commands.routineStep(task.id, task.target - task.progress), () => {})}
      />
      <span className="ro-task-name">{task.label}</span>
      {task.target > 1 && (
        <span className="ro-task-count num">
          {task.progress}/{task.target}
        </span>
      )}
    </li>
  );
}

/** The routine at a glance, above the game: what caps soon, what is left. */
export function RoutineOverlay() {
  const routine = useRoutine();
  const petsView = usePets();
  const { settings, t } = useSettings();
  const { language } = settings;
  const panel = useRef<HTMLElement>(null);
  useFitWindowHeight(panel);
  useNow(30_000);
  const now = Date.now();

  // Only what needs you soon: full already, or full within a day.
  const reserves = (routine?.reserves ?? [])
    .filter((r) => r.reading && urgency(r) - now <= DAY_MS)
    .sort((a, b) => urgency(a) - urgency(b));
  // Followed bosses with something to say: up, or a time to come back.
  const bosses = (routine?.bosses ?? [])
    .filter((b) => b.favorite && b.status.kind !== "unknown")
    .sort(bySoonest)
    .slice(0, MAX_BOSSES);
  const pets = (petsView?.pets ?? []).filter((pet) => pet.favorite && pet.needed !== null);
  const pending = (routine?.tasks ?? []).filter((task) => task.progress < task.target);
  const daily = pending.filter((task) => task.group === "daily");
  const weekly = pending.filter((task) => task.group !== "daily");

  return (
    <main ref={panel} className="routine-overlay surface" data-click-through={settings.clickThrough || undefined}>
      <header className="ro-head" data-tauri-drag-region>
        <span className="ro-title" data-tauri-drag-region>
          {t("tab.routine")}
        </span>
        {routine && (
          <span className="ro-reset num" data-tauri-drag-region>
            {t("routine.dailyReset")} {formatRemaining(routine.nextDailyResetMs - now, language)}
          </span>
        )}
        <span className="ro-controls">
          <button type="button" className="icon-btn" title={t("tab.routine")} onClick={() => void commands.openHub("routine", null)}>
            <Icon name="details" />
          </button>
          <button type="button" className="icon-btn" title={t("meter.hide")} onClick={() => void commands.setRoutineOverlay(false)}>
            <Icon name="close" />
          </button>
        </span>
      </header>

      {reserves.length > 0 && (
        <ul className="ro-reserves">
          {reserves.map((reserve) => {
            const reading = reserve.reading;
            if (!reading) return null;
            const full = reading.fullSinceMs != null;
            return (
              <li key={reserve.id} className="ro-reserve" data-full={full || undefined}>
                <span className="ro-reserve-name">{reserve.label}</span>
                <span className="num ro-reserve-amount">
                  {reading.amount}/{reserve.cap}
                </span>
                <span className="num ro-reserve-when">
                  {full ? t("routine.full") : reading.fullAtMs != null ? formatRemaining(reading.fullAtMs - now, language) : ""}
                </span>
              </li>
            );
          })}
        </ul>
      )}

      {bosses.length > 0 && (
        <section>
          <h2 className="ro-section">{t("bosses.followed")}</h2>
          <ul className="ro-bosses">
            {bosses.map((boss) => (
              <li key={boss.id} className="ro-boss" data-up={boss.status.kind === "up" || undefined}>
                <span className="ro-reserve-name">{boss.label}</span>
                <span className="num ro-boss-when">{shortStatus(boss, now, t, language)}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {pets.length > 0 && (
        <section>
          <h2 className="ro-section">{t("pets.followed")}</h2>
          <ul className="ro-bosses">
            {pets.map((pet) => (
              <li key={pet.id} className="ro-boss">
                <span className="ro-reserve-name">{petName(pet, t)}</span>
                <span className="num ro-boss-when">
                  {pet.souls}/{pet.needed} <span className="ro-pet-level">{t("pets.level")} {pet.level + 1}</span>
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}

      {daily.length > 0 && (
        <section>
          <h2 className="ro-section">{t("routine.daily")}</h2>
          <ol className="ro-tasks">
            {daily.map((task) => (
              <OverlayTask key={task.id} task={task} locked={settings.clickThrough} />
            ))}
          </ol>
        </section>
      )}
      {weekly.length > 0 && (
        <section>
          <h2 className="ro-section">
            {t("routine.weekly")}
            {weekly.length > MAX_WEEKLY && <span className="ro-more num">+{weekly.length - MAX_WEEKLY}</span>}
          </h2>
          <ol className="ro-tasks">
            {weekly.slice(0, MAX_WEEKLY).map((task) => (
              <OverlayTask key={task.id} task={task} locked={settings.clickThrough} />
            ))}
          </ol>
        </section>
      )}
      {routine && daily.length === 0 && weekly.length === 0 && <p className="ro-empty">{t("routine.allDone")}</p>}
    </main>
  );
}
