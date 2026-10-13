import { useMemo, useRef } from "react";

import type { TimerView } from "../../lib/bindings";
import { useTracker } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { useFitWindowHeight } from "../../lib/windowFit";
import { TimerTile } from "./TimerTile";
import { useNow } from "./useNow";

const timerKey = (t: TimerView) => `${t.kind}:${t.id}`;

export function TrackerView() {
  const tracker = useTracker();
  const { settings, t, save } = useSettings();
  const { showBuffs, showCooldowns, iconSize, hidden } = settings.tracker;
  const now = useNow();
  const panel = useRef<HTMLElement>(null);
  useFitWindowHeight(panel);
  // Snapshots arrive every 250 ms; countdowns run from when each arrived.
  const receivedAt = useMemo(() => (tracker ? performance.now() : 0), [tracker]);

  const hide = (timer: TimerView) =>
    void save({ ...settings, tracker: { ...settings.tracker, hidden: [...hidden, timerKey(timer)] } });

  const rows = [
    showBuffs ? { id: "buffs", label: t("settings.showBuffs"), timers: tracker?.buffs ?? [] } : null,
    showCooldowns ? { id: "cooldowns", label: t("settings.showCooldowns"), timers: tracker?.cooldowns ?? [] } : null,
  ].flatMap((row) => (row ? [{ ...row, timers: row.timers.filter((timer) => !hidden.includes(timerKey(timer))) }] : []));

  const empty = rows.every((row) => row.timers.length === 0);

  return (
    <main ref={panel} className="tracker" data-click-through={settings.clickThrough || undefined} data-tauri-drag-region>
      {rows.map((row) =>
        row.timers.length > 0 ? (
          <section key={row.id} className="tracker-row" aria-label={row.label} data-tauri-drag-region>
            {row.timers.map((timer) => (
              <TimerTile
                key={timerKey(timer)}
                timer={timer}
                remainingMs={Math.max(0, timer.remainingMs - (now - receivedAt))}
                size={iconSize}
                hint={t("meter.hide")}
                onHide={settings.clickThrough ? null : () => hide(timer)}
              />
            ))}
          </section>
        ) : null,
      )}
      {empty && <p className="tracker-empty" data-tauri-drag-region>{t("settings.tracker")}</p>}
    </main>
  );
}
