import { useState } from "react";

import { commands } from "../../lib/bindings";
import { formatTime } from "../../lib/format";
import { usePets } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { openLink, run } from "../routine/run";
import { Checkbox, Choice } from "../settings/controls";
import { useNow } from "../tracker/useNow";
import { PetRow } from "./PetRow";
import { inProgress, type PetFilter, type PetSort, selectPets } from "./text";

const CATALOG_EDIT_URL = "https://github.com/Wany917/aether-meter/edit/main/catalog/pets.json";
const SOURCE_URL = "https://github.com/taengu/A2Tools-Plugins/tree/main/plugins/pet-souls";

/** Every pet's level and souls, as the game reports them, and where its souls drop. */
export function PetsTab() {
  const view = usePets();
  const { settings, t } = useSettings();
  const [filter, setFilter] = useState<PetFilter>("progress");
  const [sort, setSort] = useState<PetSort>("closest");
  const [genus, setGenus] = useState("all");
  const [search, setSearch] = useState("");
  const [openId, setOpenId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  useNow(60_000);
  const now = Date.now();

  if (!view) return <div className="tab-loading" />;

  const hidden = new Set(view.regions.filter((r) => r.hidden).map((r) => r.id));
  const shown = selectPets(view.pets, { filter, sort, search, genus, hidden }, t);
  const done = view.pets.filter((pet) => !inProgress(pet)).length;
  const genusName = new Map(view.genera.map((g) => [g.id, g.name]));
  const filters = [
    ["progress", t("pets.filter.progress")],
    ["all", t("pets.filter.all")],
    ["followed", t("pets.filter.followed")],
  ] as const;
  const sorts = [
    ["closest", t("pets.sort.closest")],
    ["left", t("pets.sort.left")],
    ["recent", t("pets.sort.recent")],
    ["id", t("pets.sort.id")],
  ] as const;
  const genera = [["all", t("pets.genus.all")] as const, ...view.genera.map((g) => [g.id, g.name] as const)];

  const setRegion = (id: string, farmed: boolean) => {
    const next = new Set(hidden);
    if (farmed) next.delete(id);
    else next.add(id);
    run(commands.petsRegions([...next]), setError);
  };

  return (
    <div className="routine pets">
      <header className="routine-bar">
        <div className="routine-resets">
          {view.serverId !== 0 && (
            <span>
              {t("pets.server")} <strong>{view.serverName ?? view.serverId}</strong>
            </span>
          )}
          {view.syncedAtMs !== null && (
            <span>
              {t("pets.syncedAt")} <strong className="num">{formatTime(view.syncedAtMs, settings.language)}</strong>
            </span>
          )}
          {view.pets.length > 0 && (
            <span>
              <strong className="num">
                {done}/{view.pets.length}
              </strong>{" "}
              {t("pets.done")}
            </span>
          )}
        </div>
        <div className="routine-toggles pets-controls">
          <input className="field pets-search" placeholder={t("pets.search")} value={search} onChange={(e) => setSearch(e.currentTarget.value)} />
          <Choice value={filter} options={filters} label={t("pets.filter")} onChange={setFilter} />
          {view.genera.length > 0 && <Choice value={genus} options={genera} label={t("pets.genus")} onChange={setGenus} />}
          <Choice value={sort} options={sorts} label={t("pets.sort")} onChange={setSort} />
        </div>
      </header>

      {error && (
        <p className="settings-error" role="alert" onClick={() => setError(null)}>
          {error}
        </p>
      )}

      {view.regions.length > 0 && (
        <div className="pets-regions" title={t("pets.regionsHint")}>
          <span className="pets-regions-label">{t("pets.regions")}</span>
          {view.regions.map((region) => (
            <label key={region.id} className="inline-check">
              <Checkbox checked={!region.hidden} label={region.name} onChange={(on) => setRegion(region.id, on)} />
              {region.name}
            </label>
          ))}
        </div>
      )}

      <p className="routine-hint bosses-hint">{view.syncedAtMs === null ? t("pets.notSynced") : t("pets.hint")}</p>

      {shown.length === 0 ? (
        view.pets.length > 0 && <p className="routine-hint">{t("pets.empty")}</p>
      ) : (
        <ol className="task-list pet-grid">
          {shown.map((pet) => (
            <PetRow
              key={pet.id}
              pet={pet}
              genus={pet.genus === null ? null : (genusName.get(pet.genus) ?? null)}
              maxLevel={view.maxLevel}
              now={now}
              open={openId === pet.id}
              regions={view.regions}
              onToggle={() => setOpenId((current) => (current === pet.id ? null : pet.id))}
              onError={setError}
            />
          ))}
        </ol>
      )}

      <footer className="routine-foot">
        <div className="routine-catalog">
          <span>
            {t("routine.catalog")}, {t("routine.revision")} {view.catalogRevision}
          </span>
          <button type="button" className="text-btn" onClick={() => openLink(SOURCE_URL)}>
            {t("pets.credit")}
          </button>
          <button type="button" className="text-btn" onClick={() => openLink(CATALOG_EDIT_URL)}>
            {t("pets.suggest")}
          </button>
        </div>
      </footer>
    </div>
  );
}
