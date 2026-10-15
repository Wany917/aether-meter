import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRef } from "react";

import { commands } from "../../lib/bindings";
import { useMeter } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { useFitWindowHeight } from "../../lib/windowFit";
import { MeterFooter } from "./MeterFooter";
import { MyStanding } from "./MyStanding";
import { PlayerRow } from "./PlayerRow";
import { StatusPanel } from "./StatusPanel";
import { TargetHeader } from "./TargetHeader";

/** Row height at scale 1, in px; rows are placed by index so reordering slides. */
const ROW_HEIGHT = 27;

export function MeterView() {
  const meter = useMeter();
  const { settings, t } = useSettings();
  const players = meter?.players ?? [];
  const top = players[0]?.damage ?? 0;
  const fighting = players.some((p) => p.damage > 0);
  const rowHeight = ROW_HEIGHT * settings.meter.scale;
  const panel = useRef<HTMLElement>(null);
  useFitWindowHeight(panel);
  const myIndex = players.findIndex((p) => p.isLocal);
  const me = myIndex >= 0 ? players[myIndex] : undefined;
  // Out of the visible rows: pinned below them, like the game's "My Rank".
  const pinned = me && myIndex >= settings.meter.maxRows ? me : undefined;

  return (
    <main
      ref={panel}
      className="meter surface"
      data-click-through={settings.clickThrough || undefined}
      data-cp={settings.meter.showCombatPower || undefined}
    >
      <TargetHeader meter={meter} />
      {meter && fighting ? (
        <div className="meter-columns" aria-hidden="true">
          <span>{t("meter.colPlayer")}</span>
          {settings.meter.showCombatPower && <span title={t("settings.showCombatPower")}>{t("meter.combatPower")}</span>}
          <span>{t("meter.colDps")}</span>
          <span className="col-total">{t("meter.colTotal")}</span>
          <span>%</span>
        </div>
      ) : null}
      {meter && fighting ? (
        <div className="meter-rows" style={{ maxHeight: settings.meter.maxRows * rowHeight + 4 }}>
          <ol className="meter-rows-track" style={{ height: players.length * rowHeight }}>
            {players.map((row, index) => (
              <li key={row.actorId} style={{ transform: `translateY(${index * rowHeight}px)` }}>
                <PlayerRow
                  row={row}
                  rank={index + 1}
                  fill={top > 0 ? row.damage / top : 0}
                  onOpen={() => void commands.openHub("details", row.actorId)}
                />
              </li>
            ))}
          </ol>
        </div>
      ) : (
        meter && <StatusPanel status={meter.status} />
      )}
      {fighting && pinned && (
        <div className="meter-pinned">
          <PlayerRow row={pinned} rank={myIndex + 1} fill={top > 0 ? pinned.damage / top : 0} onOpen={() => void commands.openHub("details", pinned.actorId)} />
        </div>
      )}
      {fighting && me && players.length > 1 && <MyStanding players={players} index={myIndex} />}
      {meter && <MeterFooter meter={meter} />}
      <span
        className="resize-grip"
        aria-hidden="true"
        onPointerDown={(e) => {
          e.preventDefault();
          void getCurrentWindow().startResizeDragging("East");
        }}
      />
    </main>
  );
}
