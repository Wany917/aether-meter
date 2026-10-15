import { useEffect, useState } from "react";

import { commands, type PetSourcesView, type RegionView } from "../../lib/bindings";
import { formatChance } from "../../lib/format";
import { useSettings } from "../../lib/settings";

/** Where a pet's souls drop: the monsters in the player's regions, then the chests. */
export function PetSources({ id, regions }: { id: number; regions: RegionView[] }) {
  const { settings, t } = useSettings();
  const language = settings.language;
  const [sources, setSources] = useState<PetSourcesView | null | undefined>(undefined);

  useEffect(() => {
    let live = true;
    void commands.petSources(id).then((found) => {
      if (live) setSources(found);
    });
    return () => {
      live = false;
    };
  }, [id, language]);

  if (sources === undefined) return <div className="pet-sources" />;
  if (sources === null) {
    return (
      <div className="pet-sources">
        <p className="pet-sources-note">{t("pets.sources.unknown")}</p>
      </div>
    );
  }

  const hidden = new Set(regions.filter((r) => r.hidden).map((r) => r.id));
  const drops = sources.drops.filter((drop) => !hidden.has(drop.region));
  const elsewhere = regions.filter((r) => r.hidden && sources.drops.some((drop) => drop.region === r.id));

  return (
    <div className="pet-sources">
      {drops.length > 0 ? (
        <table className="pet-drops">
          <thead>
            <tr>
              <th>{t("pets.sources.monster")}</th>
              <th className="num">{t("pets.level")}</th>
              <th className="num">{t("pets.sources.chance")}</th>
              <th>{t("pets.sources.where")}</th>
              <th className="num">{t("pets.sources.spawns")}</th>
              <th>{t("pets.sources.teleport")}</th>
            </tr>
          </thead>
          <tbody>
            {drops.map((drop) => (
              <tr key={`${drop.monster}|${drop.region}|${drop.zones.join()}`}>
                <td className="pet-drop-monster">{drop.monster}</td>
                <td className="num">{drop.level}</td>
                <td className="num">{formatChance(drop.chance, language)}</td>
                <td>
                  {drop.zones.join(", ")}
                  <span className="pet-drop-map">, {drop.map}</span>
                </td>
                <td className="num">{drop.spawns}</td>
                <td>
                  {drop.teleport && (
                    <>
                      <span className="num">
                        {t("pets.sources.number")}
                        {drop.teleport.number}
                      </span>{" "}
                      {drop.teleport.name}
                      <span className="pet-drop-map num">, {drop.teleport.distanceM} m</span>
                    </>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        elsewhere.length === 0 && <p className="pet-sources-note">{t("pets.sources.noDrop")}</p>
      )}
      {elsewhere.length > 0 && (
        <p className="pet-sources-note">
          {t("pets.sources.elsewhere")} {elsewhere.map((r) => r.name).join(", ")} ({t("pets.sources.unchecked")})
        </p>
      )}
      {sources.chests.length > 0 && (
        <div className="pet-chests">
          <span className="pet-chests-title">{t("pets.sources.chests")}</span>
          {sources.chests.map((chest) => (
            <span key={chest.name}>
              {chest.name} <span className="num">{formatChance(chest.chance, language)}</span>
              <span className="pet-drop-map">, {chest.from.join(", ")}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
