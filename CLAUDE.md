# Aether (AION 2 DPS overlay)

Tauri 2 desktop app + Rust engine + React/TS UI, Bun for JS tooling. GPL-3.0.

## Commands
- `bun run dev:demo`: run with a synthetic fight (the only mode that works on macOS)
- `bun run test`: `cargo test --workspace` + `tsc --noEmit`
- `bun run bindings`: regenerate `apps/overlay/src/lib/bindings.ts` (tauri-specta). Never edit it by hand.
- `bun run sync:gamedata`: refresh game data + class marks from A2Tools at the pinned rev
- `bun run sync:pets [ref]`: pet names, genera and soul sources from A2Tools-Plugins (GPL-3.0);
  bumps `catalog/pets.json` revision only when names or genera changed
- `bun run probe <capture.pcapng> --find <n>`: find which game record carries a value (packet discovery)

## Architecture
- `crates/engine`: no Tauri. `Engine` facade; `Feed` trait with `LiveFeed` (parser) and `DemoFeed`.
  Parser = A2Tools crate (`a2tools` in workspace deps, `default-features = false`).
  `model.rs` is the UI contract; parser structs never reach the frontend.
  Target mode Auto (`feed/auto.rs`): the parser's boss mode while that boss fight is going on
  (held 60 s after its last hit while alive, 10 s once dead), else a second calculator in
  "lastHitByMe". Only the first calculator to ask sees a zone reset: the second restarts
  itself when its target is gone from `DataStorage`.
- `crates/routine`: no Tauri. Content comes from `catalog/routine.json` (community catalog, validated by
  `Catalog::parse` with the same rules as `catalog/routine.schema.json`); player state is separate.
  Any catalog change bumps `revision`; clients fetch newer revisions from GitHub main.
  Bosses live in the same catalog: kills come from the engine (`Engine::take_kills`, death
  records read off the frame tap), schedules are on a server clock (`zone.rs`, Berlin for EU).
  Odyle Energy comes from the parser (`DataStorage::odyle_energy`, via `Engine::take_odyle`
  when it changes) into the `odyle-energy` reserve as a game reading (`record_game_reading`):
  basic is the counter, charged is `refills` outside the cap.
- `crates/pets`: no Tauri. Pet collections per server from the engine's pet records
  (`Engine::take_pets`); names, genera and souls per level from `catalog/pets.json`
  (community-fetched); where souls drop from `catalog/pet-sources.json` (bundled only,
  too big for the 512 KB fetch limit). Regions of the other side are hidden by default
  (side from the server number).
- `apps/overlay/src-tauri`: thin shell. Commands delegate to `actions.rs` or the engine.
  `gameasset://` scheme serves NCSOFT font/icons from a disk cache (allow-listed routes only).
- `apps/overlay/src`: `modules/<feature>/`, shared `ui/`, `lib/` (ipc, i18n, format).

## Design rules (from the owner)
- Match the game's sober UI: near-black navy, white/grey text, hairlines, square checkboxes,
  diamond markers, the arc of light under active tabs.
- Rows are rounded plates (`.plate`, `ui/ui.css`). Bars are solid, in the class's emblem colour sunk
  into the navy (`.plate-bar`); the local player's bar is at full strength with a light frame.
- Cyan only for the "You" label (the game's "My Rank"). The local name is white bold.
- No translucent or saturated colour, no glows, no pills/badges, no iOS toggles, no left accent bars.
- Keep the hub dense: sizes close to the meter's (rows ~27 px, titles ≤ 20 px).
- French UI copy: no em/en dashes.
