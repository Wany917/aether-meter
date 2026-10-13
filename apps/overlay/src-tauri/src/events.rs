//! Events pushed to the frontend. Typed on both sides through tauri-specta.

use aether_engine::model::{MeterSnapshot, TrackerSnapshot};
use aether_pets::PetsView;
use aether_routine::RoutineView;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct MeterUpdated(pub MeterSnapshot);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct TrackerUpdated(pub TrackerSnapshot);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct SettingsChanged(pub Settings);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct RoutineChanged(pub RoutineView);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct PetsChanged(pub PetsView);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HubTab {
    Details,
    History,
    Routine,
    Bosses,
    Pets,
    Settings,
}

/// Bring the hub to a tab, optionally on one player of the live fight.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct HubNavigate {
    pub tab: HubTab,
    pub actor_id: Option<i32>,
}
