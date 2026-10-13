# Notices

Aether is licensed under the GNU General Public License v3.0 only (see `LICENSE`).

## Code from other projects

- **A2Tools DPS Meter** by taengu, GPL-3.0
  (<https://github.com/taengu/A2Tools-DPS-Meter>). The packet parser and combat
  aggregation are used as a Cargo dependency at a pinned revision. The live
  capture (`crates/engine/src/capture/pcap.rs`), connection locking
  (`crates/engine/src/dispatch.rs`) and skill icon resolver
  (`crates/engine/src/icons.rs`) are adapted from it. The game data in
  `apps/overlay/src-tauri/resources/gamedata/` and the class marks in
  `apps/overlay/src/assets/classes/` come from its repository. The layout of
  the pet collection records decoded in `crates/engine/src/pets.rs` is the one
  its pet soul tracker reads (decoder tests in its `plugins/decoder.rs`).
- **RATmeter** by Kuroukihime, GPL-3.0
  (<https://github.com/Kuroukihime/AIon2-Dps-Meter>): the layout of the skill
  cooldown record decoded in `crates/engine/src/tracker.rs`.

## Community catalogs

`catalog/routine.json` and `catalog/pets.json` hold facts (entry counts, caps,
recharge rates, respawn rules, souls per level) gathered from public guides,
each listed in its `sources`. Guide links point to
[aion2.help](https://www.aion2.help), a fan-made wiki; none of its text is
copied here.

## Game assets

AION 2, its class emblems, skill and buff icons and its typeface are
© NCSOFT Corporation and are **not** covered by this project's license. The
class marks in `apps/overlay/src/assets/classes/` are included to identify
classes. Icons, emblems and the typeface are fetched at runtime from NCSOFT's
public CDN and cached on the user's machine; they are never redistributed.

Aether is an unofficial fan project, not affiliated with or endorsed by NCSOFT.
