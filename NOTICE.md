# Notices

Aether is licensed under the GNU General Public License v3.0 only (see `LICENSE`).

## Code from other projects

- **A2Tools DPS Meter** by taengu, GPL-3.0
  (<https://github.com/taengu/A2Tools-DPS-Meter>). The packet parser and combat
  aggregation are used as a Cargo dependency at a pinned revision, and ship
  in Aether's binaries. Portions of A2Tools derive from AION2-DPS-Meter
  (AION2Meter4J) by TK-open-public
  (<https://github.com/TK-open-public/Aion2-Dps-Meter>), under the MIT License
  reproduced at the end of this file. The live
  capture (`crates/engine/src/capture/pcap.rs`), connection locking
  (`crates/engine/src/dispatch.rs`) and skill icon resolver
  (`crates/engine/src/icons.rs`) are adapted from it. The game data in
  `apps/overlay/src-tauri/resources/gamedata/` and the class marks in
  `apps/overlay/src/assets/classes/` come from its repository. The layout of
  the pet collection records decoded in `crates/engine/src/pets.rs` is the one
  its pet soul tracker reads (decoder tests in its `plugins/decoder.rs`).
- **A2Tools-Plugins** by taengu, GPL-3.0
  (<https://github.com/taengu/A2Tools-Plugins>). The pet names, genera and
  soul sources in `catalog/pets.json` and `catalog/pet-sources.json` come from
  its Pet Soul Tracker (`plugins/pet-souls/pets.js` and `sources.js`), which
  reads them from the game client's own tables and maps; genera are from
  [wikily.gg](https://wikily.gg/aion-2/pets). `bun run sync:pets` converts
  them; the commit used is in `catalog/pet-sources.json` (`origin`).
- **RATmeter** by Kuroukihime, GPL-3.0
  (<https://github.com/Kuroukihime/AIon2-Dps-Meter>): the layout of the skill
  cooldown record decoded in `crates/engine/src/tracker.rs`.

## Community catalogs

`catalog/routine.json` and `catalog/pets.json` hold facts (entry counts, caps,
recharge rates, respawn rules, souls per level) gathered from public guides,
each listed in its `sources`, besides the pet data credited above. Guide links point to
[aion2.help](https://www.aion2.help), a fan-made wiki; none of its text is
copied here.

## Game assets

AION 2, its class emblems, skill and buff icons and its typeface are
© NCSOFT Corporation and are **not** covered by this project's license. The
class marks in `apps/overlay/src/assets/classes/` are included to identify
classes. Icons, emblems and the typeface are fetched at runtime from NCSOFT's
public CDN and cached on the user's machine; they are never redistributed.

Aether is an unofficial fan project, not affiliated with or endorsed by NCSOFT.

## MIT License (TK-open-public)

Covers the portions of A2Tools DPS Meter derived from AION2-DPS-Meter.

```text
MIT License

Copyright (c) 2026 TK-open-public

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
