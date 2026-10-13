import { convertFileSrc } from "@tauri-apps/api/core";
import type { ClassId } from "./bindings";

import assassin from "../assets/classes/assassin.png";
import brawler from "../assets/classes/brawler.png";
import chanter from "../assets/classes/chanter.png";
import cleric from "../assets/classes/cleric.png";
import elementalist from "../assets/classes/elementalist.png";
import gladiator from "../assets/classes/gladiator.png";
import ranger from "../assets/classes/ranger.png";
import sorcerer from "../assets/classes/sorcerer.png";
import templar from "../assets/classes/templar.png";

/** Served by the app's `gameasset://` scheme, from NCSOFT's CDN through a disk cache. */
const SCHEME = "gameasset";
const CDN_ICONS = "https://assets.playnccdn.com/static-aion2-gamedata/resources/";

function gameAsset(path: string): string {
  return convertFileSrc(path, SCHEME);
}

/** A CDN icon URL from the engine, routed through the local cache. */
export function iconSource(url: string): string {
  return url.startsWith(CDN_ICONS) ? gameAsset(`icon/${url.slice(CDN_ICONS.length)}`) : url;
}

/** The small class mark the game shows next to names (party finder, rankings). */
export const CLASS_MARKS: Record<ClassId, string | null> = {
  gladiator,
  templar,
  ranger,
  assassin,
  sorcerer,
  elementalist,
  cleric,
  chanter,
  brawler,
  unknown: null,
};

/** The large framed class emblem from the official character pages. */
export function classEmblem(id: ClassId): string | null {
  return id === "unknown" ? null : gameAsset(`emblem/${id}.png`);
}

let fontRequested = false;

/** Load AION 2's own typeface. The UI falls back to Segoe UI until it is in. */
export function loadGameFont(): void {
  if (fontRequested) return;
  fontRequested = true;
  // Medium carries body text, so it is registered as the normal weight.
  const faces = [
    ["Light", "300"],
    ["Medium", "400"],
    ["Bold", "700"],
  ] as const;
  for (const [file, weight] of faces) {
    const face = new FontFace("Aion2", `url("${gameAsset(`font/${file}`)}") format("woff2")`, { weight, display: "swap" });
    face
      .load()
      .then((loaded) => document.fonts.add(loaded))
      .catch(() => {
        // Offline on first launch: keep the fallback font.
      });
  }
}
