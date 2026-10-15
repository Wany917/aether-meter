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

export interface PetQuery {
  filter: PetFilter;
  sort: PetSort;
  search: string;
  /** A genus id, or `all`. */
  genus: string;
  /** The regions the player left out. */
  hidden: ReadonlySet<string>;
}

/** Its souls drop where the player farms, or nowhere known. */
export function farmable(pet: PetView, hidden: ReadonlySet<string>): boolean {
  return pet.regions.length === 0 || pet.regions.some((region) => !hidden.has(region));
}

/** The pets a query leaves, followed ones first; a followed pet ignores the regions. */
export function selectPets(pets: PetView[], query: PetQuery, t: Translate): PetView[] {
  const search = query.search.trim().toLowerCase();
  const { filter, genus, hidden } = query;
  return pets
    .filter((pet) => (filter === "all" ? true : filter === "followed" ? pet.favorite : inProgress(pet)))
    .filter((pet) => genus === "all" || pet.genus === genus)
    .filter((pet) => pet.favorite || farmable(pet, hidden))
    .filter((pet) => !search || petName(pet, t).toLowerCase().includes(search) || String(pet.id).includes(search))
    .sort((a, b) => Number(b.favorite) - Number(a.favorite) || SORTS[query.sort](a, b) || a.id - b.id);
}
