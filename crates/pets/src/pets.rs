use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::catalog::{PetCatalog, localize};
use crate::collection::Collection;
use crate::error::PetsError;
use crate::sources::{PetSources, Side};
use crate::view::{GenusView, PetSourcesView, PetView, PetsView};

const MAX_NAME_CHARS: usize = 40;

/// What the game said about the collection, as the engine read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// The whole collection at login.
    List {
        levels: Vec<(u32, u32)>,
        souls: Vec<(u32, u32)>,
    },
    Soul {
        pet: u32,
        amount: u32,
    },
    Level {
        pet: u32,
        level: u32,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PetState {
    /// Server id -> its collection; 0 for a server the game never named.
    pub servers: BTreeMap<u16, Collection>,
    /// The server played last: the tab shows its collection.
    pub current_server: u16,
    pub favorites: Vec<u32>,
    /// Pet id -> the player's own name for it.
    pub names: BTreeMap<u32, String>,
    /// The regions the player left out; `None` until they choose, which
    /// leaves out the other side's maps.
    pub hidden_regions: Option<Vec<String>>,
}

/// The catalog, where souls drop, and the collections the game reported.
pub struct Pets {
    catalog: PetCatalog,
    sources: PetSources,
    state: PetState,
}

impl Pets {
    pub fn new(catalog: PetCatalog, sources: PetSources, state: PetState) -> Self {
        Self {
            catalog,
            sources,
            state,
        }
    }

    pub fn state(&self) -> &PetState {
        &self.state
    }

    pub fn current_server(&self) -> u16 {
        self.state.current_server
    }

    pub fn catalog_revision(&self) -> u32 {
        self.catalog.revision
    }

    /// Take a newer catalog revision; an older or equal one is ignored.
    pub fn offer_catalog(&mut self, catalog: PetCatalog) -> bool {
        if catalog.revision <= self.catalog.revision {
            return false;
        }
        self.catalog = catalog;
        true
    }

    /// File records under `server_id`, or the server played last when the
    /// game has not named one. Returns whether anything changed.
    pub fn apply(&mut self, server_id: u16, records: Vec<Record>, now: i64) -> bool {
        if records.is_empty() {
            return false;
        }
        if server_id != 0 {
            self.state.current_server = server_id;
        }
        let catalog = &self.catalog;
        let collection = self
            .state
            .servers
            .entry(self.state.current_server)
            .or_default();
        for record in records {
            match record {
                Record::List { levels, souls } => collection.sync(&levels, &souls, now),
                Record::Soul { pet, amount } => {
                    collection.gain(pet, amount, now, |level| catalog.needed(level));
                }
                Record::Level { pet, level } => collection.level_up(pet, level),
            }
        }
        true
    }

    fn side(&self) -> Option<Side> {
        Side::of_server(self.state.current_server)
    }

    fn hidden_regions(&self) -> Vec<String> {
        self.state
            .hidden_regions
            .clone()
            .unwrap_or_else(|| self.sources.other_side(self.side()))
    }

    /// The collection of the server played last; `server_name` names it.
    pub fn view(&self, language: &str, server_name: Option<String>) -> PetsView {
        let empty = Collection::default();
        let collection = self
            .state
            .servers
            .get(&self.state.current_server)
            .unwrap_or(&empty);
        // The pets the game mentioned, the catalog's (to see what is left to
        // collect) and the followed ones.
        let ids: BTreeSet<u32> = collection
            .pets()
            .chain(self.catalog.pets.iter().map(|p| p.id))
            .chain(self.state.favorites.iter().copied())
            .collect();
        let max_level = self.catalog.max_level();
        let pets = ids
            .into_iter()
            .filter_map(|id| {
                let def = self.catalog.pet(id);
                let special = def.is_some_and(|d| d.special);
                let custom = self.state.names.get(&id);
                let (level, souls) = (collection.level(id), collection.souls(id));
                let favorite = self.state.favorites.contains(&id);
                // Out of reach until its maps open, unless the player has some already.
                if def.is_some_and(|d| d.unreleased) && level == 0 && souls == 0 && !favorite {
                    return None;
                }
                let to_max = if special {
                    0
                } else {
                    let left: u32 = self
                        .catalog
                        .souls_per_level
                        .iter()
                        .skip(level as usize)
                        .sum();
                    left.saturating_sub(souls)
                };
                Some(PetView {
                    id,
                    name: custom
                        .cloned()
                        .or_else(|| def.and_then(|d| localize(&d.name, language))),
                    custom_name: custom.is_some(),
                    // As the game reports it: a special pet comes whole, but
                    // only once the player has it.
                    level,
                    souls,
                    needed: if special {
                        None
                    } else {
                        self.catalog.needed(level)
                    },
                    to_max,
                    favorite,
                    special,
                    genus: def.and_then(|d| d.genus.clone()),
                    regions: self.sources.regions_of(id),
                    last_gain_ms: collection
                        .last_gain
                        .filter(|(pet, _)| *pet == id)
                        .map(|(_, at)| at),
                })
            })
            .collect();
        let genera = self
            .catalog
            .genera
            .iter()
            .filter_map(|(id, name)| {
                localize(name, language).map(|name| GenusView {
                    id: id.clone(),
                    name,
                })
            })
            .collect();
        PetsView {
            server_id: self.state.current_server,
            server_name,
            synced_at_ms: collection.synced_at,
            max_level,
            souls_per_level: self.catalog.souls_per_level.clone(),
            pets,
            catalog_revision: self.catalog.revision,
            side: self.side(),
            genera,
            regions: self.sources.regions(language, &self.hidden_regions()),
        }
    }

    /// Where a pet's souls come from, with the Kibelisks the player's side uses.
    pub fn sources(&self, id: u32, language: &str) -> Option<PetSourcesView> {
        self.sources.view(id, language, self.side())
    }

    /// The regions the player farms in: pets that only drop elsewhere are not listed.
    pub fn set_hidden_regions(&mut self, hidden: Vec<String>) {
        let mut hidden: Vec<String> = hidden
            .into_iter()
            .filter(|id| self.sources.is_region(id))
            .collect();
        hidden.sort();
        hidden.dedup();
        self.state.hidden_regions = Some(hidden);
    }

    pub fn set_favorite(&mut self, id: u32, on: bool) {
        self.state.favorites.retain(|f| *f != id);
        if on {
            self.state.favorites.push(id);
        }
    }

    /// The player's name for a pet; an empty one goes back to the catalog's.
    pub fn rename(&mut self, id: u32, name: &str) -> Result<(), PetsError> {
        let name = name.trim();
        if name.is_empty() {
            self.state.names.remove(&id);
            return Ok(());
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(PetsError::InvalidName);
        }
        self.state.names.insert(id, name.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pets() -> Pets {
        Pets::new(
            PetCatalog::bundled(),
            PetSources::bundled(),
            PetState::default(),
        )
    }

    fn pet(view: &PetsView, id: u32) -> PetView {
        view.pets.iter().find(|p| p.id == id).cloned().unwrap()
    }

    #[test]
    fn collections_are_kept_per_server() {
        let mut p = pets();
        p.apply(
            1304,
            vec![Record::List {
                levels: vec![(1097, 2)],
                souls: vec![(1097, 30)],
            }],
            0,
        );
        p.apply(
            2014,
            vec![Record::List {
                levels: vec![(1097, 1)],
                souls: vec![],
            }],
            1,
        );
        assert_eq!(
            pet(&p.view("fr", None), 1097).level,
            1,
            "the server played last"
        );
        // A soul with no server named goes to the one played last.
        p.apply(
            0,
            vec![Record::Soul {
                pet: 1097,
                amount: 1,
            }],
            2,
        );
        assert_eq!(p.state().servers[&2014].souls(1097), 1);
        assert_eq!(p.state().servers[&1304].souls(1097), 30);
    }

    #[test]
    fn the_view_counts_what_is_left_to_the_top_level() {
        let mut p = pets();
        p.apply(
            1304,
            vec![Record::List {
                levels: vec![(1097, 2)],
                souls: vec![(1097, 30), (1016, 3)],
            }],
            0,
        );
        let view = p.view("fr", None);
        let (bee, newcomer) = (pet(&view, 1097), pet(&view, 1016));
        assert_eq!((bee.needed, bee.to_max), (Some(75), 45));
        assert_eq!(
            (newcomer.level, newcomer.needed, newcomer.to_max),
            (0, Some(5), 102)
        );
        assert_eq!(bee.name.as_deref(), Some("Karnif"));
        assert_eq!(bee.genus.as_deref(), Some("fera"));
    }

    #[test]
    fn names_and_favorites_are_the_players() {
        let mut p = pets();
        p.rename(1097, "  Abeille  ").unwrap();
        p.set_favorite(1097, true);
        let bee = pet(&p.view("fr", None), 1097);
        assert_eq!(
            (bee.name.as_deref(), bee.custom_name, bee.favorite),
            (Some("Abeille"), true, true)
        );
        assert!(p.rename(1097, &"x".repeat(41)).is_err());
        p.rename(1097, "").unwrap();
        assert_eq!(
            pet(&p.view("fr", None), 1097).name.as_deref(),
            Some("Karnif")
        );
    }

    #[test]
    fn unreleased_pets_wait_until_the_player_has_some() {
        let mut p = pets();
        let unreleased = PetCatalog::bundled()
            .pets
            .iter()
            .find(|d| d.unreleased)
            .map(|d| d.id)
            .unwrap();
        let listed = |p: &Pets| p.view("en", None).pets.iter().any(|v| v.id == unreleased);
        assert!(!listed(&p));
        p.apply(
            1304,
            vec![Record::Soul {
                pet: unreleased,
                amount: 1,
            }],
            0,
        );
        assert!(listed(&p));
    }

    #[test]
    fn the_other_sides_maps_are_hidden_until_the_player_chooses() {
        let mut p = pets();
        let hidden = |p: &Pets| -> Vec<String> {
            p.view("en", None)
                .regions
                .into_iter()
                .filter(|r| r.hidden)
                .map(|r| r.id)
                .collect()
        };
        assert!(hidden(&p).is_empty(), "no side before a server is seen");
        p.apply(
            2014,
            vec![Record::List {
                levels: vec![],
                souls: vec![],
            }],
            0,
        );
        assert_eq!(p.view("en", None).side, Some(Side::Asmodian));
        assert_eq!(hidden(&p), vec!["Verteron".to_string()]);
        p.set_hidden_regions(vec!["Fire Temple".into(), "Atlantis".into()]);
        assert_eq!(hidden(&p), vec!["Fire Temple".to_string()]);
        assert!(p.sources(1002, "en").is_some());
    }
}
