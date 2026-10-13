import { type CSSProperties, useRef, useState } from "react";

import { commands, type PetView } from "../../lib/bindings";
import { formatTime } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { Icon } from "../../ui/Icon";
import { run } from "../routine/run";
import { levelProgress, petName } from "./text";

/** A farmed pet stays marked this long after its last soul. */
const FARMING_MS = 30 * 60_000;

export function PetRow({ pet, maxLevel, now, onError }: { pet: PetView; maxLevel: number; now: number; onError: (m: string) => void }) {
  const { settings, t } = useSettings();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const cancelled = useRef(false);

  const startRename = () => {
    setDraft(pet.customName ? (pet.name ?? "") : "");
    setEditing(true);
  };
  // Enter and Escape both leave through blur, so a name is saved once.
  const finish = () => {
    setEditing(false);
    const save = !cancelled.current;
    cancelled.current = false;
    if (save && draft.trim() !== (pet.customName ? pet.name : "")) run(commands.petsRename(pet.id, draft), onError);
  };

  const farming = pet.lastGainMs !== null && now - pet.lastGainMs < FARMING_MS;
  const top = pet.needed === null;
  const detail = pet.special
    ? t("pets.special")
    : top
      ? t("pets.max")
      : `${t("pets.left")} ${pet.toMax} ${t("pets.souls")} ${t("pets.toMax")} ${maxLevel}`.trim();

  return (
    <li className="pet plate" data-top={top || undefined} style={{ "--fill": `${levelProgress(pet) * 100}%` } as CSSProperties}>
      <span className="plate-bar" aria-hidden="true" />
      <button
        type="button"
        className="boss-star"
        aria-pressed={pet.favorite}
        title={pet.favorite ? t("pets.unfollow") : t("pets.follow")}
        onClick={() => run(commands.petsFavorite(pet.id, !pet.favorite), onError)}
      >
        <Icon name="star" fill={pet.favorite ? "currentColor" : "none"} />
      </button>
      <span className="pet-level num">
        {t("pets.level")} {pet.level}
      </span>
      <div className="task-text">
        {editing ? (
          <input
            className="pet-rename"
            autoFocus
            maxLength={40}
            value={draft}
            placeholder={pet.name ?? petName(pet, t)}
            onChange={(e) => setDraft(e.currentTarget.value)}
            onBlur={finish}
            onKeyDown={(e) => {
              if (e.key === "Escape") cancelled.current = true;
              if (e.key === "Enter" || e.key === "Escape") e.currentTarget.blur();
            }}
          />
        ) : (
          <span className="task-name" data-unnamed={pet.name === null || undefined} title={t("pets.rename")} onDoubleClick={startRename}>
            {petName(pet, t)}
          </span>
        )}
        <span className="task-note">
          {detail}
          {farming && pet.lastGainMs !== null && ` · ${t("pets.farming")} ${formatTime(pet.lastGainMs, settings.language)}`}
        </span>
      </div>
      {!top && (
        <span className="pet-souls num">
          {pet.souls}
          <span className="pet-needed">/{pet.needed}</span>
        </span>
      )}
    </li>
  );
}
