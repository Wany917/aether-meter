import { useEffect, useState } from "react";

import { commands, type TimerLabel } from "../../lib/bindings";
import { useSettings } from "../../lib/settings";
import { GameIcon } from "../../ui/GameIcon";

const keyOf = (label: TimerLabel) => `${label.kind}:${label.id}`;

/** Timers hidden from the tracker, each with a way back. */
export function HiddenTimers({ keys, onRestore }: { keys: string[]; onRestore: (key: string) => void }) {
  const { t } = useSettings();
  const [labels, setLabels] = useState<TimerLabel[]>([]);

  useEffect(() => {
    let alive = true;
    void commands.describeTimers(keys).then((found) => alive && setLabels(found));
    return () => {
      alive = false;
    };
  }, [keys]);

  return (
    <ul className="hidden-timers">
      {labels.map((label) => (
        <li key={keyOf(label)}>
          <GameIcon candidates={label.icons} label={label.name} size={22} />
          <span className="hidden-name">{label.name}</span>
          <button type="button" className="text-btn" onClick={() => onRestore(keyOf(label))}>
            {t("settings.restore")}
          </button>
        </li>
      ))}
    </ul>
  );
}
