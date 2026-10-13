import type { PlayerRow } from "../../lib/bindings";
import { formatAmount, formatOrdinal } from "../../lib/format";
import { useTweened } from "../../lib/motion";
import { useSettings } from "../../lib/settings";
import { displayName } from "./names";

/**
 * Where you stand, in one line: "You · 2nd of 6 · 4.4K/s behind Lyria".
 * The game's "My Rank" in spirit, with the gap that matters.
 */
export function MyStanding({ players, index }: { players: PlayerRow[]; index: number }) {
  const { settings, t } = useSettings();
  const { language } = settings;
  const { numberFormat, nameDisplay } = settings.meter;
  const me = players[index];
  const rival = index > 0 ? players[index - 1] : players[1];
  const gap = useTweened(me && rival ? Math.abs(rival.dps - me.dps) : 0);
  if (!me) return null;

  return (
    <div className="my-standing">
      <span className="my-label">{t("me.you")}</span>
      <span className="num">
        {formatOrdinal(index + 1, language)} {t("me.of")} {players.length}
      </span>
      {rival && (
        <span className="my-gap">
          <span className="num">{formatAmount(gap, numberFormat, language)}/s</span> {t(index > 0 ? "me.behind" : "me.ahead")}{" "}
          {displayName(rival.name, rival.class, nameDisplay, language)}
        </span>
      )}
    </div>
  );
}
