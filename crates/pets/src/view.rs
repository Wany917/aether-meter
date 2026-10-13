//! What the pets tab shows.

use serde::{Deserialize, Serialize};

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
    pub last_gain_ms: Option<i64>,
}
