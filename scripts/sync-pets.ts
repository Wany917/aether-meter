/**
 * Pulls pet names, genera and soul sources from taengu's Pet Soul Tracker
 * (A2Tools-Plugins, GPL-3.0) into Aether's catalogs:
 *
 * - `catalog/pets.json`: names and genera. Its revision goes up when they
 *   change, and installed apps fetch it from GitHub.
 * - `catalog/pet-sources.json`: the monsters and chests each pet's souls come
 *   from, shipped with the app.
 *
 *   bun run sync:pets [ref]
 */
import { join } from "node:path";

const REPO = "taengu/A2Tools-Plugins";
const PLUGIN = "plugins/pet-souls";
const ROOT = join(import.meta.dir, "..");
const PETS_OUT = join(ROOT, "catalog/pets.json");
const SOURCES_OUT = join(ROOT, "catalog/pet-sources.json");
const PLUGIN_URL = `https://github.com/${REPO}/tree/main/${PLUGIN}`;

/** The checks `crates/pets/src/catalog.rs` makes, so no installed app rejects the file. */
const MAX_NAME_CHARS = 60;

/** Each side's own maps, as the plugin names them (by English region). */
const SIDE_REGIONS: Record<string, "elyos" | "asmodian"> = {
  Poeta: "elyos",
  Verteron: "elyos",
  Eltnen: "elyos",
  Ishalgen: "asmodian",
  Altgard: "asmodian",
  Morheim: "asmodian",
};

type Localized = Record<string, string>;

interface Teleport {
  m: number;
  name: string;
  teleport: number;
}

interface UpstreamDrop {
  chance: string;
  level: number;
  map: string;
  npc: string;
  region: string;
  spawns: number;
  zones: string[];
  kibelisk?: Teleport;
  bySide?: { elyos: Teleport; asmodian: Teleport };
}

interface UpstreamSource {
  chests: { chance: string; from: string[]; name: string }[];
  drops: UpstreamDrop[];
  unreleased?: boolean;
}

interface PetCatalog {
  $schema: string;
  revision: number;
  updated: string;
  soulsPerLevel: number[];
  sources: string[];
  genera?: Record<string, Localized>;
  pets: { id: number; name: Localized; genus?: string; special?: boolean; unreleased?: boolean }[];
}

async function github(path: string, accept: string): Promise<Response> {
  const res = await fetch(`https://api.github.com/repos/${REPO}/${path}`, {
    headers: { Accept: accept, "User-Agent": "aether-meter-sync" },
  });
  if (!res.ok) throw new Error(`${path}: ${res.status} ${res.statusText}`);
  return res;
}

async function download(sha: string, file: string): Promise<string> {
  const res = await fetch(`https://raw.githubusercontent.com/${REPO}/${sha}/${PLUGIN}/${file}`);
  if (!res.ok) throw new Error(`${file}: ${res.status} ${res.statusText}`);
  return res.text();
}

/**
 * The object literal assigned to `const name`. The plugin's files are data in
 * JavaScript form: they are turned into JSON and parsed, never run.
 */
function literal(code: string, name: string): unknown {
  const head = `const ${name} = `;
  const start = code.indexOf(head);
  if (start < 0) throw new Error(`${name} is missing upstream`);
  const end = code.indexOf("\n};", start);
  if (end < 0) throw new Error(`${name} has no end`);
  const json = code
    .slice(start + head.length, end + 2)
    .replace(/^\s*\/\/.*$/gm, "")
    .replace(/^(\s*)([A-Za-z0-9_-]+):/gm, '$1"$2":')
    .replace(/,(\s*[}\]])/g, "$1");
  return JSON.parse(json);
}

function fail(why: string): never {
  throw new Error(`upstream data changed shape: ${why}`);
}

function localized(value: unknown, what: string): Localized {
  if (typeof value !== "object" || value === null) fail(`${what} is not a text table`);
  const text = value as Record<string, unknown>;
  if (typeof text.en !== "string") fail(`${what} has no English text`);
  for (const [language, name] of Object.entries(text)) {
    if (typeof name !== "string" || name.trim() === "") fail(`${what} (${language}) is empty`);
  }
  return text as Localized;
}

/** "2.44%" as 2.44. */
function percent(chance: string, what: string): number {
  const value = Number(chance.replace(/%$/, ""));
  if (!chance.endsWith("%") || !Number.isFinite(value) || value <= 0 || value > 100) fail(`${what} chance ${chance}`);
  return value;
}

function teleport(t: Teleport): { name: string; number: number; m: number } {
  return { name: t.name, number: t.teleport, m: Math.round(t.m) };
}

const ref = process.argv[2] ?? "main";
const sha = (await (await github(`commits/${ref}`, "application/vnd.github.sha")).text()).trim();
const [petsCode, sourcesCode] = await Promise.all([download(sha, "pets.js"), download(sha, "sources.js")]);

const names = literal(petsCode, "PET_NAMES") as Record<string, unknown>;
const genusOf = literal(petsCode, "PET_GENUS") as Record<string, string>;
const genusNames = literal(petsCode, "GENUS_NAMES") as Record<string, unknown>;
const upstream = literal(sourcesCode, "PET_SOURCES") as Record<string, UpstreamSource>;
const sourceText = literal(sourcesCode, "SOURCE_TEXT") as Record<string, unknown>;

const genera = Object.fromEntries(Object.entries(genusNames).map(([id, text]) => [id, localized(text, `genus ${id}`)]));
const pets = Object.entries(names)
  .map(([key, text]) => {
    const id = Number(key);
    if (!Number.isInteger(id) || id <= 0) fail(`pet id ${key}`);
    const name = localized(text, `pet ${id}`);
    const long = Object.values(name).find((n) => [...n].length > MAX_NAME_CHARS);
    if (long) fail(`pet ${id} name "${long}" is over ${MAX_NAME_CHARS} characters`);
    const genus = genusOf[key];
    if (genus !== undefined && !(genus in genera)) fail(`pet ${id} genus ${genus}`);
    return {
      id,
      ...(genus && { genus }),
      ...(genus === "special" && { special: true }),
      ...(upstream[key]?.unreleased && { unreleased: true }),
      name,
    };
  })
  .sort((a, b) => a.id - b.id);

// Sources: every text a pet's sources name, in the table once.
const used = new Set<string>();
const use = (key: string) => {
  if (!(key in sourceText)) fail(`text ${key} is missing`);
  used.add(key);
  return key;
};
const regionMaps = new Map<string, string>();
const sources = Object.fromEntries(
  Object.entries(upstream)
    .filter(([, source]) => !source.unreleased)
    .map(([key, source]) => {
      const drops = source.drops.map((d) => {
        const what = `pet ${key} drop ${d.npc}`;
        const known = regionMaps.get(d.region);
        if (known !== undefined && known !== d.map) fail(`region ${d.region} spans two maps`);
        regionMaps.set(d.region, use(d.map));
        return {
          npc: use(d.npc),
          level: d.level,
          chance: percent(d.chance, what),
          region: d.region,
          zones: d.zones.map(use),
          spawns: d.spawns,
          ...(d.kibelisk && { teleport: teleport({ ...d.kibelisk, name: use(d.kibelisk.name) }) }),
          ...(d.bySide && {
            bySide: {
              elyos: teleport({ ...d.bySide.elyos, name: use(d.bySide.elyos.name) }),
              asmodian: teleport({ ...d.bySide.asmodian, name: use(d.bySide.asmodian.name) }),
            },
          }),
        };
      });
      const chests = source.chests.map((c) => ({
        name: use(c.name),
        chance: percent(c.chance, `pet ${key} chest ${c.name}`),
        from: c.from.map(use),
      }));
      return [key, { drops, chests }];
    }),
);
// Each side's maps first, then Reshanta and the dungeons, as the plugin lists them.
const regions = [...regionMaps]
  .map(([id, map]) => ({ id, map, ...(SIDE_REGIONS[id] && { side: SIDE_REGIONS[id] }) }))
  .sort((a, b) => Number(!a.side) - Number(!b.side) || Number(a.id.includes("Reshanta")) - Number(b.id.includes("Reshanta")) || a.id.localeCompare(b.id));
const text = Object.fromEntries([...used].sort().map((key) => [key, localized(sourceText[key], `text ${key}`)]));

// The pet catalog: a new revision only when what it says changed.
const current = (await Bun.file(PETS_OUT).json()) as PetCatalog;
const changed = JSON.stringify([current.genera, current.pets]) !== JSON.stringify([genera, pets]);
const catalog: PetCatalog = {
  $schema: "./pets.schema.json",
  revision: changed ? current.revision + 1 : current.revision,
  updated: changed ? new Date().toISOString().slice(0, 10) : current.updated,
  soulsPerLevel: current.soulsPerLevel,
  sources: [...new Set([PLUGIN_URL, ...current.sources])],
  genera,
  pets,
};

/** One entry per line, so a game patch reads as a short diff. */
const block = (entries: [string, unknown][], indent = "  ") =>
  `{\n${entries.map(([k, v]) => `${indent}  ${JSON.stringify(k)}: ${JSON.stringify(v)}`).join(",\n")}\n${indent}}`;
const list = (items: unknown[]) => `[\n${items.map((v) => `    ${JSON.stringify(v)}`).join(",\n")}\n  ]`;

await Bun.write(
  PETS_OUT,
  `{
  "$schema": ${JSON.stringify(catalog.$schema)},
  "revision": ${catalog.revision},
  "updated": ${JSON.stringify(catalog.updated)},
  "soulsPerLevel": ${JSON.stringify(catalog.soulsPerLevel)},
  "sources": ${list(catalog.sources)},
  "genera": ${block(Object.entries(genera))},
  "pets": ${list(pets)}
}
`,
);
await Bun.write(
  SOURCES_OUT,
  `{
  "origin": ${JSON.stringify(`https://github.com/${REPO}/blob/${sha}/${PLUGIN}/sources.js`)},
  "regions": ${list(regions)},
  "pets": ${block(Object.entries(sources))},
  "text": ${block(Object.entries(text))}
}
`,
);

const farmable = Object.values(sources).filter((s) => s.drops.length > 0).length;
console.log(
  `${REPO}@${sha.slice(0, 7)}: ${pets.length} pets (revision ${catalog.revision}${changed ? ", changed" : ""}), ` +
    `${Object.keys(sources).length} with sources (${farmable} dropped by monsters), ${regions.length} regions, ${used.size} texts`,
);
