//! The community pet catalog (`catalog/pets.json`): what the game's records
//! leave out (names, genera) and the souls each level takes. Names and genera
//! come from taengu's Pet Soul Tracker (`bun run sync:pets`). Validated with
//! the same rules as `catalog/pets.schema.json`, so a broken update is never
//! applied.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::PetsError;

/// The catalog shipped with this build.
pub const BUNDLED: &str = include_str!("../../../catalog/pets.json");

/// Language code -> text. English is always present.
pub type Localized = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetCatalog {
    pub revision: u32,
    pub updated: String,
    /// Souls from level 0 to 1, 1 to 2...; its length is the top level.
    pub souls_per_level: Vec<u32>,
    #[serde(default)]
    pub sources: Vec<String>,
    /// Genus id -> its name, as the game shows it.
    #[serde(default)]
    pub genera: BTreeMap<String, Localized>,
    pub pets: Vec<PetDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PetDef {
    pub id: u32,
    pub name: Localized,
    /// A key of `genera` (what Pet Genus Insight counts).
    #[serde(default)]
    pub genus: Option<String>,
    /// Comes at its highest level and takes no souls.
    #[serde(default)]
    pub special: bool,
    /// Its souls only drop on maps not open yet: listed once the player has some.
    #[serde(default)]
    pub unreleased: bool,
}

impl PetCatalog {
    pub fn bundled() -> Self {
        Self::parse(BUNDLED).expect("the bundled pet catalog is validated by the tests")
    }

    pub fn parse(json: &str) -> Result<Self, PetsError> {
        let catalog: PetCatalog =
            serde_json::from_str(json).map_err(|e| PetsError::Catalog(e.to_string()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn pet(&self, id: u32) -> Option<&PetDef> {
        self.pets.iter().find(|p| p.id == id)
    }

    pub fn max_level(&self) -> u32 {
        self.souls_per_level.len() as u32
    }

    /// Souls to go from `level` to the next one; `None` at the top.
    pub fn needed(&self, level: u32) -> Option<u32> {
        self.souls_per_level.get(level as usize).copied()
    }

    fn validate(&self) -> Result<(), PetsError> {
        let invalid = |why: String| Err(PetsError::Catalog(why));
        if self.revision == 0 {
            return invalid("revision must be at least 1".into());
        }
        if self.souls_per_level.is_empty()
            || self.souls_per_level.len() > 10
            || self
                .souls_per_level
                .iter()
                .any(|s| !(1..=10_000).contains(s))
        {
            return invalid("soulsPerLevel holds 1 to 10 counts of 1 to 10000".into());
        }
        if self.sources.iter().any(|url| !url.starts_with("https://")) {
            return invalid("links must be https".into());
        }
        for (id, name) in &self.genera {
            if !valid_name(name) {
                return invalid(format!(
                    "genus {id}: names need an `en` entry and 1 to 60 characters each"
                ));
            }
        }
        let mut ids = HashSet::new();
        for pet in &self.pets {
            if pet.id == 0 || !ids.insert(pet.id) {
                return invalid(format!("pet {}: ids are positive and unique", pet.id));
            }
            if !valid_name(&pet.name) {
                return invalid(format!(
                    "pet {}: names need an `en` entry and 1 to 60 characters each",
                    pet.id
                ));
            }
            if pet
                .genus
                .as_ref()
                .is_some_and(|g| !self.genera.contains_key(g))
            {
                return invalid(format!("pet {}: its genus is not in `genera`", pet.id));
            }
        }
        Ok(())
    }
}

fn valid_name(name: &Localized) -> bool {
    name.contains_key("en")
        && name
            .values()
            .all(|n| !n.trim().is_empty() && n.chars().count() <= 60)
}

/// The text in `language`, else English.
pub fn localize(text: &Localized, language: &str) -> Option<String> {
    text.get(language).or_else(|| text.get("en")).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalog_is_valid() {
        let catalog = PetCatalog::parse(BUNDLED).unwrap();
        assert_eq!(catalog.max_level(), 3);
        assert_eq!(
            (catalog.needed(0), catalog.needed(2), catalog.needed(3)),
            (Some(5), Some(75), None)
        );
        let laborer = catalog.pet(1002).unwrap();
        assert_eq!(
            localize(&laborer.name, "fr").as_deref(),
            Some("Ouvrier Krall")
        );
        assert!(
            catalog
                .pets
                .iter()
                .all(|p| p.special == (p.genus.as_deref() == Some("special")))
        );
    }

    #[test]
    fn broken_catalogs_are_rejected() {
        let base: serde_json::Value = serde_json::from_str(BUNDLED).unwrap();
        let broken = |edit: fn(&mut serde_json::Value)| {
            let mut value = base.clone();
            edit(&mut value);
            PetCatalog::parse(&value.to_string())
        };
        assert!(broken(|v| v["soulsPerLevel"] = serde_json::json!([])).is_err());
        assert!(
            broken(|v| v["pets"] = serde_json::json!([{ "id": 1, "name": { "fr": "x" } }]))
                .is_err()
        );
        assert!(
            broken(|v| v["pets"] = serde_json::json!([
                { "id": 1, "name": { "en": "A" } }, { "id": 1, "name": { "en": "B" } }
            ]))
            .is_err()
        );
        assert!(
            broken(|v| v["pets"] = serde_json::json!([
                { "id": 7, "name": { "en": "Kerub" }, "genus": "unknown" }
            ]))
            .is_err()
        );
        assert!(
            broken(|v| v["pets"] = serde_json::json!([{ "id": 7, "name": { "en": "Kerub" } }]))
                .is_ok()
        );
    }
}
