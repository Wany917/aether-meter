import { useState } from "react";

import { formatTime } from "../../lib/format";
import { usePets } from "../../lib/ipc";
import { useSettings } from "../../lib/settings";
import { openLink } from "../routine/run";
import { Choice } from "../settings/controls";
import { useNow } from "../tracker/useNow";
import { PetRow } from "./PetRow";
import { inProgress, type PetFilter, type PetSort, selectPets } from "./text";

const CATALOG_EDIT_URL = "https://github.com/Wany917/aether-meter/edit/main/catalog/pets.json";

/** Every pet's level and souls, as the game reports them. */
export function PetsTab() {
  const view = usePets();
  const { settings, t } = useSettings();
  const [filter, setFilter] = useState<PetFilter>("progress");
  const [sort, setSort] = useState<PetSort>("closest");
  const [search, setSearch] = useState("");
  const [error, setError] = useState<string | null>(null);
  useNow(60_000);
  const now = Date.now();

  if (!view) return <div className="tab-loading" />;

  const shown = selectPets(view.pets, filter, sort, search, t);
  const done = view.pets.filter((pet) => !inProgress(pet)).length;
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
          <Choice value={sort} options={sorts} label={t("pets.sort")} onChange={setSort} />
        </div>
      </header>

      {error && (
        <p className="settings-error" role="alert" onClick={() => setError(null)}>
          {error}
        </p>
      )}

      <p className="routine-hint bosses-hint">{view.syncedAtMs === null ? t("pets.notSynced") : t("pets.hint")}</p>

      {shown.length === 0 ? (
        view.pets.length > 0 && <p className="routine-hint">{t("pets.empty")}</p>
      ) : (
        <ol className="task-list pet-grid">
          {shown.map((pet) => (
            <PetRow key={pet.id} pet={pet} maxLevel={view.maxLevel} now={now} onError={setError} />
          ))}
        </ol>
      )}

      <footer className="routine-foot">
        <div className="routine-catalog">
          <span>
            {t("routine.catalog")}, {t("routine.revision")} {view.catalogRevision}
          </span>
          <button type="button" className="text-btn" onClick={() => openLink(CATALOG_EDIT_URL)}>
            {t("pets.suggest")}
          </button>
        </div>
      </footer>
    </div>
  );
}
