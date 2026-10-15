//! Where each pet's souls come from (`catalog/pet-sources.json`): the
//! monsters that drop them, with the nearest Kibelisk, and the chests that
//! hold them. taengu's Pet Soul Tracker reads it from the game client's own
//! tables and maps; `bun run sync:pets` brings it here, and it ships with the
//! app (a game patch means a new build, not a catalog update).

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::catalog::{Localized, localize};
use crate::error::PetsError;
use crate::view::{ChestView, DropView, PetSourcesView, RegionView, TeleportView};

pub const BUNDLED_SOURCES: &str = include_str!("../../../catalog/pet-sources.json");

/// A side of the war: each has maps of its own, and Kibelisks of its own in the Abyss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Elyos,
    Asmodian,
}

impl Side {
    /// From the server number: 1xxx are Elyos servers, 2xxx Asmodian ones.
    pub fn of_server(id: u16) -> Option<Self> {
        match id / 1000 {
            1 => Some(Self::Elyos),
            2 => Some(Self::Asmodian),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PetSources {
    regions: Vec<RegionDef>,
    pets: BTreeMap<u32, SourceDef>,
    /// Text key -> the game's text in each language.
    text: BTreeMap<String, Localized>,
}

#[derive(Debug, Deserialize)]
struct RegionDef {
    /// The region's English name, as the player's choice of regions is saved.
    id: String,
    map: String,
    /// The side the map belongs to; `None` for the Abyss and the dungeons.
    #[serde(default)]
    side: Option<Side>,
}

#[derive(Debug, Deserialize)]
struct SourceDef {
    drops: Vec<DropDef>,
    chests: Vec<ChestDef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DropDef {
    npc: String,
    level: u32,
    chance: f32,
    region: String,
    zones: Vec<String>,
    spawns: u32,
    #[serde(default)]
    teleport: Option<TeleportDef>,
    /// In the Abyss each side has its own Kibelisks.
    #[serde(default)]
    by_side: Option<BySide>,
}

#[derive(Debug, Deserialize)]
struct BySide {
    elyos: TeleportDef,
    asmodian: TeleportDef,
}

#[derive(Debug, Deserialize)]
struct TeleportDef {
    name: String,
    /// Its number on the in-game map.
    number: u32,
    /// From the middle of the spawns, in metres.
    m: u32,
}

#[derive(Debug, Deserialize)]
struct ChestDef {
    name: String,
    chance: f32,
    /// Where the chest is earned.
    from: Vec<String>,
}

impl PetSources {
    pub fn bundled() -> Self {
        Self::parse(BUNDLED_SOURCES).expect("the bundled pet sources are validated by the tests")
    }

    pub fn parse(json: &str) -> Result<Self, PetsError> {
        let sources: PetSources =
            serde_json::from_str(json).map_err(|e| PetsError::Sources(e.to_string()))?;
        sources.validate()?;
        Ok(sources)
    }

    /// Every region, each side's maps first.
    pub fn regions(&self, language: &str, hidden: &[String]) -> Vec<RegionView> {
        self.regions
            .iter()
            .map(|r| RegionView {
                id: r.id.clone(),
                name: self.text(&r.map, language),
                side: r.side,
                hidden: hidden.contains(&r.id),
            })
            .collect()
    }

    /// The regions of the other side, which a player cannot farm.
    pub fn other_side(&self, side: Option<Side>) -> Vec<String> {
        let Some(side) = side else {
            return Vec::new();
        };
        self.regions
            .iter()
            .filter(|r| r.side.is_some_and(|s| s != side))
            .map(|r| r.id.clone())
            .collect()
    }

    pub fn is_region(&self, id: &str) -> bool {
        self.regions.iter().any(|r| r.id == id)
    }

    /// The regions where a pet's souls drop, in the regions' order.
    pub fn regions_of(&self, pet: u32) -> Vec<String> {
        let Some(source) = self.pets.get(&pet) else {
            return Vec::new();
        };
        self.regions
            .iter()
            .filter(|r| source.drops.iter().any(|d| d.region == r.id))
            .map(|r| r.id.clone())
            .collect()
    }

    /// A pet's sources in `language`, with the Kibelisks `side` can use;
    /// the lowest monsters first.
    pub fn view(&self, pet: u32, language: &str, side: Option<Side>) -> Option<PetSourcesView> {
        let source = self.pets.get(&pet)?;
        let mut drops: Vec<DropView> = source
            .drops
            .iter()
            .map(|d| {
                let teleport = match (&d.by_side, side) {
                    (Some(both), Some(Side::Elyos)) => Some(&both.elyos),
                    (Some(both), Some(Side::Asmodian)) => Some(&both.asmodian),
                    _ => d.teleport.as_ref(),
                };
                DropView {
                    monster: self.text(&d.npc, language),
                    level: d.level,
                    chance: d.chance,
                    region: d.region.clone(),
                    map: self
                        .regions
                        .iter()
                        .find(|r| r.id == d.region)
                        .map(|r| self.text(&r.map, language))
                        .unwrap_or_else(|| d.region.clone()),
                    zones: d.zones.iter().map(|z| self.text(z, language)).collect(),
                    spawns: d.spawns,
                    teleport: teleport.map(|t| TeleportView {
                        name: self.text(&t.name, language),
                        number: t.number,
                        distance_m: t.m,
                    }),
                }
            })
            .collect();
        drops.sort_by_key(|d| (d.level, std::cmp::Reverse(d.spawns)));
        let chests = source
            .chests
            .iter()
            .map(|c| ChestView {
                name: self.text(&c.name, language),
                chance: c.chance,
                from: c.from.iter().map(|f| self.text(f, language)).collect(),
            })
            .collect();
        Some(PetSourcesView { drops, chests })
    }

    fn text(&self, key: &str, language: &str) -> String {
        self.text
            .get(key)
            .and_then(|t| localize(t, language))
            .unwrap_or_else(|| key.to_string())
    }

    fn validate(&self) -> Result<(), PetsError> {
        let invalid = |why: String| Err(PetsError::Sources(why));
        let mut regions = HashSet::new();
        for region in &self.regions {
            if !regions.insert(region.id.as_str()) {
                return invalid(format!("region {} is listed twice", region.id));
            }
        }
        let missing = |key: &String| !self.text.get(key).is_some_and(|t| t.contains_key("en"));
        for region in &self.regions {
            if missing(&region.map) {
                return invalid(format!("region {}: no text for its map", region.id));
            }
        }
        for (pet, source) in &self.pets {
            for drop in &source.drops {
                let teleports = drop
                    .teleport
                    .iter()
                    .chain(drop.by_side.iter().flat_map(|b| [&b.elyos, &b.asmodian]));
                if !regions.contains(drop.region.as_str())
                    || missing(&drop.npc)
                    || drop.zones.iter().any(missing)
                    || teleports.into_iter().any(|t| missing(&t.name))
                    || !(0.0..=100.0).contains(&drop.chance)
                {
                    return invalid(format!("pet {pet}: drop {} is incomplete", drop.npc));
                }
            }
            for chest in &source.chests {
                if missing(&chest.name)
                    || chest.from.iter().any(missing)
                    || !(0.0..=100.0).contains(&chest.chance)
                {
                    return invalid(format!("pet {pet}: chest {} is incomplete", chest.name));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_sources_are_valid() {
        let sources = PetSources::parse(BUNDLED_SOURCES).unwrap();
        assert!(sources.pets.len() > 150);
        assert_eq!(sources.regions_of(1002), vec!["Verteron".to_string()]);

        let laborer = sources.view(1002, "fr", Some(Side::Elyos)).unwrap();
        assert_eq!(laborer.drops[0].monster, "Soldat armé Krall");
        assert_eq!(laborer.drops[0].level, 21);
        let teleport = laborer.drops[0].teleport.as_ref().unwrap();
        assert_eq!(
            (teleport.number, teleport.name.as_str(), teleport.distance_m),
            (19, "Colonne effondrée", 106)
        );
        assert!(
            laborer.chests[0]
                .name
                .starts_with("Coffre d'âme de familier")
        );
    }

    #[test]
    fn each_side_has_its_own_kibelisks_in_the_abyss() {
        let sources = PetSources::bundled();
        let (pet, _) = sources
            .pets
            .iter()
            .find(|(_, s)| s.drops.iter().any(|d| d.by_side.is_some()))
            .unwrap();
        let near = |side| {
            let view = sources.view(*pet, "en", side).unwrap();
            let drop = view
                .drops
                .into_iter()
                .find(|d| d.region.contains("Reshanta"))
                .unwrap();
            drop.teleport.unwrap().name
        };
        assert_ne!(near(Some(Side::Elyos)), near(Some(Side::Asmodian)));
    }

    #[test]
    fn the_other_sides_maps_are_the_ones_to_hide() {
        let sources = PetSources::bundled();
        assert_eq!(Side::of_server(1304), Some(Side::Elyos));
        assert_eq!(Side::of_server(2014), Some(Side::Asmodian));
        assert_eq!(Side::of_server(0), None);
        assert_eq!(
            sources.other_side(Some(Side::Elyos)),
            vec!["Altgard".to_string()]
        );
        assert!(sources.other_side(None).is_empty());
    }

    #[test]
    fn broken_sources_are_rejected() {
        let base: serde_json::Value = serde_json::from_str(BUNDLED_SOURCES).unwrap();
        let broken = |edit: fn(&mut serde_json::Value)| {
            let mut value = base.clone();
            edit(&mut value);
            PetSources::parse(&value.to_string())
        };
        assert!(broken(|v| v["pets"]["1002"]["drops"][0]["region"] = "Elysea".into()).is_err());
        assert!(
            broken(|v| v["pets"]["1002"]["drops"][0]["npc"] = "String_unknown".into()).is_err()
        );
        assert!(broken(|v| v["pets"]["1002"]["chests"][0]["chance"] = 140.into()).is_err());
    }
}
