# Aether

A clean DPS meter overlay for **AION 2**, styled after the game's own windows.

Aether reads the game's network traffic passively (no memory reading, no
injection) and shows your party's damage on a small always-on-top overlay, with
a per-skill breakdown, saved boss fights, and a buff and cooldown tracker.

- **Meter**: party DPS and share on the current target, boss HP, fight timer,
  ping. Click-through when locked.
- **Details**: every player's skills: damage, hits, critical, back and perfect
  rates, average and max hit.
- **History**: boss fights saved automatically, browsable afterwards.
- **Buffs & cooldowns**: your timed buffs and skill cooldowns as icon tiles.
- **Routine**: daily and weekly checklists that clear themselves at the game's
  reset, banked counters (Odyle Energy, Nightmare, Conquest...) projected to
  their cap so nothing goes to waste, and reminders before a reset or a cap.
- **Bosses**: field boss respawn timers that start on their own when a boss dies
  near you (kills seen elsewhere can be entered), fixed-time Reshanta bosses on
  the European server clock, and the ones you follow on the overlay with a
  reminder before they return.
- **Pets**: every pet's level and souls toward the next one, read from the
  game at login and counted as souls drop, per server like the game's own
  collection; where each pet's souls drop (monsters, spawns, nearest
  Kibelisk, chests), filtered to the regions you farm; followed pets on the
  overlay.
- French and English interface; skill and monster names in ten languages.

## Install (Windows)

1. Install [Npcap](https://npcap.com) with **WinPcap API-compatible Mode** checked.
2. Download the installer from [Releases](../../releases) and run it.
3. Play AION 2 in **borderless windowed** mode so the overlay can sit above it.

Aether locks onto the game's connection by itself once you are in the world.
Shortcuts (configurable): `Ctrl+Shift+L` lock or unlock the overlays,
`Ctrl+Shift+R` reset the fight, `Ctrl+Shift+H` show or hide the overlays.

Third-party meters are not endorsed by NCSOFT. Aether only reads traffic that
reaches your own machine, but use it at your own discretion.

## Develop

Requirements: [Bun](https://bun.sh), Rust 1.93+, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
bun install
bun run dev:demo        # synthetic fight, works on any OS (macOS included)
bun run dev             # live capture (Windows with Npcap, or Linux with libpcap)
bun run test            # Rust tests + TypeScript check
bun run bindings        # regenerate the typed TS bindings after changing commands or models
bun run build           # installers in target/release/bundle
```

Replay a capture recorded with Wireshark on the game PC:

```sh
bun --cwd apps/overlay tauri dev -- -- --replay path/to/fight.pcapng --speed 2
```

### Layout

```
crates/engine/          capture, connection locking, parsing, UI model, history
crates/routine/         checklists, banked counters, boss timers, reminders (no Tauri)
crates/pets/            pet soul tracker: collections per server, names (no Tauri)
catalog/                community-maintained content: routine, bosses, pets (+ where souls drop)
apps/overlay/src-tauri/ desktop shell: windows, tray, hotkeys, game-asset cache
apps/overlay/src/       React UI, one page per window (meter, tracker, hub)
scripts/                game data and pet data sync
```

The parser is [A2Tools DPS Meter](https://github.com/taengu/A2Tools-DPS-Meter)'s,
pinned by revision in the workspace `Cargo.toml`. After a game patch, bump the
`rev`, run `bun run sync:gamedata`, and run the tests. Pet names and where
their souls drop come from taengu's [Pet Soul Tracker](https://github.com/taengu/A2Tools-Plugins/tree/main/plugins/pet-souls):
once it is updated, `bun run sync:pets` brings them in.

## Routine catalog: help keep it right

What the Routine and Bosses tabs list (tasks, counts, recharge rates, caps,
reset hour, boss NPC codes and respawn rules) lives in [`catalog/routine.json`](catalog/routine.json), not in the code.
Clients fetch the latest revision from this repository, so a merged fix reaches
every player without a new release.

To fix or add something after a game update:

1. Edit `catalog/routine.json` (the format is described by
   [`catalog/routine.schema.json`](catalog/routine.schema.json)).
2. Bump `revision` by one and set `updated` to today.
3. Put your source in `sources`, link the matching
   [aion2.help guide](https://www.aion2.help/guide) in `guide` when there is one,
   and set `verified` to `true` only once you checked it in game.
4. Run `cargo test -p aether-routine` (it validates the catalog), then open a pull request.

## Decode more of the game

Hidden cubes, Odyle Energy, quest progress and entry counters are not decoded
by any meter yet. To find the record that carries one of them, record it from
the app:

1. In Settings, **Help decode the game**: pick what you will do (open a hidden
   cube, spend Odyle Energy...) and note the value in game (12 keys).
2. **Record**, do the thing once or twice, come back, enter the values you saw
   before and after (12, 11), then **Stop and save**.
3. Read the file it shows:

```sh
bun run probe aether-cube-1791936000.jsonl
```

The values saved with the recording are searched for; the opcodes listed as
holding every value, at the same offset, are the record to decode.
`--dump <opcode>` prints all of its frames. A Wireshark capture (`.pcapng`) works
too, with `--find <value>` for each value.

A recording holds what the game sent you (names of players nearby, chat):
share the result in an issue, and the file itself only with a maintainer you
trust.

## License

GPL-3.0-only. See `NOTICE.md` for third-party code and for NCSOFT's assets,
which this license does not cover.
