/**
 * Pulls the game data files and class icons from A2Tools at the exact parser
 * revision pinned in the workspace Cargo.toml, so names and icon tables always
 * match what the parser decodes.
 *
 *   bun run sync:gamedata
 */
import { mkdir, rm } from "node:fs/promises";
import { dirname, join } from "node:path";

const REPO = "taengu/A2Tools-DPS-Meter";
const ROOT = join(import.meta.dir, "..");
const DATA_OUT = join(ROOT, "apps/overlay/src-tauri/resources/gamedata");
const ICONS_OUT = join(ROOT, "apps/overlay/src/assets/classes");

/** A2Tools names its class icons in Korean. */
const CLASS_ICONS: Record<string, string> = {
  "검성": "gladiator",
  "수호성": "templar",
  "궁성": "ranger",
  "살성": "assassin",
  "마도성": "sorcerer",
  "정령성": "elementalist",
  "치유성": "cleric",
  "호법성": "chanter",
  "권성": "brawler",
};

interface TreeEntry {
  path: string;
  type: "blob" | "tree" | "commit";
}

async function pinnedRevision(): Promise<string> {
  const manifest = await Bun.file(join(ROOT, "Cargo.toml")).text();
  const rev = manifest.match(/^a2tools\s*=.*\brev\s*=\s*"([0-9a-f]{40})"/m)?.[1];
  if (!rev) throw new Error("No pinned `rev` for a2tools in Cargo.toml");
  return rev;
}

async function listFiles(rev: string): Promise<string[]> {
  const res = await fetch(`https://api.github.com/repos/${REPO}/git/trees/${rev}?recursive=1`, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "aether-meter-sync" },
  });
  if (!res.ok) throw new Error(`GitHub tree request failed: ${res.status} ${res.statusText}`);
  const body = (await res.json()) as { tree: TreeEntry[]; truncated: boolean };
  if (body.truncated) throw new Error("GitHub returned a truncated tree");
  return body.tree.filter((e) => e.type === "blob").map((e) => e.path);
}

async function download(rev: string, path: string, to: string): Promise<void> {
  const url = `https://raw.githubusercontent.com/${REPO}/${rev}/${encodeURI(path)}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${path}: ${res.status} ${res.statusText}`);
  await mkdir(dirname(to), { recursive: true });
  await Bun.write(to, await res.arrayBuffer());
}

const rev = await pinnedRevision();
const files = await listFiles(rev);

// A2Tools' own UI strings: the overlay ships its own.
const dataFiles = files.filter((p) => p.startsWith("src/data/") && p.endsWith(".json") && !p.startsWith("src/data/i18n/ui/"));
const iconFiles = Object.keys(CLASS_ICONS).map((ko) => `public/assets/${ko}.png`);
const missing = iconFiles.filter((p) => !files.includes(p));
if (missing.length > 0) throw new Error(`Class icons missing upstream: ${missing.join(", ")}`);

await rm(DATA_OUT, { recursive: true, force: true });
await Promise.all([
  ...dataFiles.map((p) => download(rev, p, join(DATA_OUT, p.slice("src/data/".length)))),
  ...Object.entries(CLASS_ICONS).map(([ko, id]) => download(rev, `public/assets/${ko}.png`, join(ICONS_OUT, `${id}.png`))),
]);

await Bun.write(
  join(DATA_OUT, "SOURCE.md"),
  `# Game data\n\nFrom [${REPO}](https://github.com/${REPO}) at \`${rev}\` (\`src/data/\`, GPL-3.0).\n` +
    "Regenerate with `bun run sync:gamedata` after bumping the parser revision.\n",
);

console.log(`Synced ${dataFiles.length} data files and ${iconFiles.length} class icons from ${REPO}@${rev.slice(0, 7)}`);
