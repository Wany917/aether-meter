//! What the pets tab shows.

use serde::{Deserialize, Serialize};

use crate::sources::Side;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PetsView {
    /// The server the collection is from; 0 until the game names it.
    pub server_id: u16,
    pub server_name: Option<String>,
    /// When the game last sent the whole collection; `None` before any login seen.
    pub synced_at_ms: Option<i64>,
    pub max_level: u32,
    pub souls_per_level: Vec<u32>,
    pub pets: Vec<PetView>,
    pub catalog_revision: u32,
    /// The player's side, from the server number.
    pub side: Option<Side>,
    pub genera: Vec<GenusView>,
    /// Where souls drop, and whether the player farms there.
    pub regions: Vec<RegionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GenusView {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RegionView {
    /// The region's English name.
    pub id: String,
    pub name: String,
    pub side: Option<Side>,
    /// The player left it out: pets that only drop there are not listed.
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PetView {
    pub id: u32,
    /// The player's name for it, else the catalog's; `None` when neither has one.
    pub name: Option<String>,
    /// The name is the player's own.
    pub custom_name: bool,
    pub level: u32,
    /// Souls toward the next level.
    pub souls: u32,
    /// Souls the next level takes; `None` at the top level.
    pub needed: Option<u32>,
    /// Souls still to collect up to the top level.
    pub to_max: u32,
    pub favorite: bool,
    /// Comes at its highest level and takes no souls.
    pub special: bool,
    pub genus: Option<String>,
    /// The regions where monsters drop its souls; empty when none is known.
    pub regions: Vec<String>,
    pub last_gain_ms: Option<i64>,
}

/// Where a pet's souls come from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PetSourcesView {
    pub drops: Vec<DropView>,
    pub chests: Vec<ChestView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DropView {
    pub monster: String,
    pub level: u32,
    /// In percent; finite, as the sources are validated.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub chance: f32,
    /// The region's id, to filter by the player's regions.
    pub region: String,
    pub map: String,
    pub zones: Vec<String>,
    pub spawns: u32,
    /// The nearest Kibelisk the player's side can use.
    pub teleport: Option<TeleportView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TeleportView {
    pub name: String,
    /// Its number on the in-game map.
    pub number: u32,
    /// From the middle of the spawns.
    pub distance_m: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChestView {
    pub name: String,
    /// In percent; finite, as the sources are validated.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub chance: f32,
    /// Where the chest is earned.
    pub from: Vec<String>,
}
