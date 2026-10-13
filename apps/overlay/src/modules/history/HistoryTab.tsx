import { useEffect, useState } from "react";

import { commands, type FightSummary } from "../../lib/bindings";
import { formatAmount, formatDay, formatDuration, formatTime } from "../../lib/format";
import { className, type Translate } from "../../lib/i18n";
import { unwrap } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { ClassMark } from "../../ui/ClassMark";
import { Icon } from "../../ui/Icon";

interface Day {
  key: string;
  label: string;
  fights: FightSummary[];
}

function groupByDay(fights: FightSummary[], language: string, t: Translate): Day[] {
  const startOfToday = new Date().setHours(0, 0, 0, 0);
  const days = new Map<string, Day>();
  for (const fight of fights) {
    const date = new Date(fight.startedAtMs);
    const key = date.toDateString();
    const dayStart = new Date(fight.startedAtMs).setHours(0, 0, 0, 0);
    const label =
      dayStart === startOfToday
        ? t("history.today")
        : dayStart === startOfToday - 86_400_000
          ? t("history.yesterday")
          : formatDay(fight.startedAtMs, language);
    const day = days.get(key) ?? { key, label, fights: [] };
    day.fights.push(fight);
    days.set(key, day);
  }
  return [...days.values()];
}

export function HistoryTab({ active, onOpen }: { active: boolean; onOpen: (id: string) => void }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const [fights, setFights] = useState<FightSummary[]>();
  const [confirming, setConfirming] = useState<string | null>(null);

  useEffect(() => {
    if (active) void commands.listFights().then(setFights);
  }, [active]);

  const remove = async (id: string) => {
    if (confirming !== id) {
      setConfirming(id);
      return;
    }
    setConfirming(null);
    try {
      await unwrap(commands.deleteFight(id));
      setFights((current) => current?.filter((f) => f.id !== id));
    } catch {
      setFights(await commands.listFights());
    }
  };

  if (!fights) return <div className="tab-loading" />;
  if (fights.length === 0) {
    return (
      <div className="empty-state">
        <span className="status-sigil" aria-hidden="true" />
        <p className="empty-title">{t("history.empty")}</p>
        <p className="empty-hint">{t("history.emptyHint")}</p>
      </div>
    );
  }

  return (
    <div className="history">
      {groupByDay(fights, language, t).map((day) => (
        <section key={day.key} className="history-day">
          <h2 className="history-day-title">{day.label}</h2>
          <ol>
            {day.fights.map((fight) => (
              <li key={fight.id} className="history-row" onMouseLeave={() => setConfirming((c) => (c === fight.id ? null : c))}>
                <button type="button" className="history-open" onClick={() => onOpen(fight.id)}>
                  <span className="history-time num">{formatTime(fight.startedAtMs, language)}</span>
                  <span className="history-target">
                    <span className="history-name">{fight.targetName}</span>
                    {fight.isTraining && <span className="history-tag">{t("history.training")}</span>}
                    {fight.dungeon && <span className="history-dungeon">{fight.dungeon}</span>}
                  </span>
                  <span className="history-party">
                    {fight.classes.map((c, i) => (
                      <ClassMark key={`${c}${i}`} id={c} label={className(c, language)} size={18} />
                    ))}
                  </span>
                  <span className="history-duration num">{formatDuration(fight.durationMs)}</span>
                  <span className="history-damage num">{formatAmount(fight.totalDamage, settings.meter.numberFormat, language)}</span>
                </button>
                <button
                  type="button"
                  className={confirming === fight.id ? "btn btn-danger history-delete" : "icon-btn history-delete"}
                  title={t("history.delete")}
                  onClick={() => void remove(fight.id)}
                >
                  {confirming === fight.id ? t("history.confirmDelete") : <Icon name="trash" />}
                </button>
              </li>
            ))}
          </ol>
        </section>
      ))}
    </div>
  );
}
