import type { PetView } from "../../lib/bindings";
import type { Translate } from "../../lib/i18n";

export type PetFilter = "progress" | "all" | "followed";
export type PetSort = "closest" | "left" | "recent" | "id";

/** Still taking souls: below the top level, and not a special pet. */
export function inProgress(pet: PetView): boolean {
  return pet.needed !== null && !pet.special;
}

/** How far through its current level, 0 to 1. */
export function levelProgress(pet: PetView): number {
  if (pet.needed === null) return 1;
  return Math.min(1, pet.souls / pet.needed);
}

export function petName(pet: PetView, t: Translate): string {
  return pet.name ?? `${t("pets.unnamed")} ${pet.id}`;
}

const SORTS: Record<PetSort, (a: PetView, b: PetView) => number> = {
  closest: (a, b) => levelProgress(b) - levelProgress(a) || a.toMax - b.toMax,
  left: (a, b) => a.toMax - b.toMax,
  recent: (a, b) => (b.lastGainMs ?? 0) - (a.lastGainMs ?? 0),
  id: (a, b) => a.id - b.id,
};

/** The pets a filter, a search and a sort leave, followed ones first. */
export function selectPets(pets: PetView[], filter: PetFilter, sort: PetSort, search: string, t: Translate): PetView[] {
  const query = search.trim().toLowerCase();
  return pets
    .filter((pet) => (filter === "all" ? true : filter === "followed" ? pet.favorite : inProgress(pet)))
    .filter((pet) => !query || petName(pet, t).toLowerCase().includes(query) || String(pet.id).includes(query))
    .sort((a, b) => Number(b.favorite) - Number(a.favorite) || SORTS[sort](a, b) || a.id - b.id);
}
