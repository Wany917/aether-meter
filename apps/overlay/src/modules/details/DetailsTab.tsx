import { useEffect, useState } from "react";

import type { FightSource } from "../../lib/bindings";
import { useSettings } from "../../lib/settings";
import { FightHeader } from "./FightHeader";
import { PartyList } from "./PartyList";
import { PlayerPanel } from "./PlayerPanel";
import { useFight } from "./useFight";

export function DetailsTab({
  source,
  focusActor,
  active,
  onBackToLive,
}: {
  source: FightSource;
  focusActor: number | null;
  active: boolean;
  onBackToLive: () => void;
}) {
  const { t } = useSettings();
  const { fight, error } = useFight(source, active);
  const [selected, setSelected] = useState<number | null>(focusActor);

  useEffect(() => {
    if (focusActor !== null) setSelected(focusActor);
  }, [focusActor]);

  if (fight === undefined) return <div className="tab-loading" />;
  if (!fight) {
    return (
      <div className="empty-state">
        <span className="status-sigil" aria-hidden="true" />
        <p className="empty-title">{error ? t("history.loadError") : t("details.empty")}</p>
        <p className="empty-hint">{error ?? t("details.emptyHint")}</p>
      </div>
    );
  }

  const player =
    fight.players.find((p) => p.actorId === selected) ?? fight.players.find((p) => p.isLocal) ?? fight.players[0];

  return (
    <div className="details">
      <FightHeader fight={fight} onBackToLive={source.kind === "saved" ? onBackToLive : null} />
      <div className="details-body">
        <PartyList players={fight.players} selected={player?.actorId ?? null} onSelect={setSelected} />
        {player ? <PlayerPanel player={player} me={fight.players.find((p) => p.isLocal) ?? null} /> : <div />}
      </div>
    </div>
  );
}
