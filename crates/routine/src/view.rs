//! What the routine tab shows. Times are epoch ms; the UI counts down itself.

use serde::{Deserialize, Serialize};

use crate::catalog::TaskGroup;
use crate::schedule::Cadence;
use crate::state::ReminderSettings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RoutineView {
    pub now_ms: i64,
    pub next_daily_reset_ms: i64,
    pub next_weekly_reset_ms: i64,
    pub membership: bool,
    pub reminders: ReminderSettings,
    pub community_updates: bool,
    pub catalog_revision: u32,
    pub catalog_updated: String,
    pub tasks: Vec<TaskView>,
    pub reserves: Vec<ReserveView>,
    /// Catalog entries the player hid, which "show all" brings back.
    pub hidden_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub id: String,
    pub label: String,
    pub note: Option<String>,
    pub group: TaskGroup,
    pub cadence: Cadence,
    pub target: u16,
    /// Done this period; 0 after a reset.
    pub progress: u16,
    pub next_reset_ms: i64,
    pub guide: Option<String>,
    /// Added by the player, not from the catalog.
    pub custom: bool,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReserveView {
    pub id: String,
    pub label: String,
    pub note: Option<String>,
    pub guide: Option<String>,
    pub verified: bool,
    pub cap: u32,
    pub regen: u32,
    pub every_hours: u16,
    /// `None` until the player enters a value.
    pub reading: Option<ReserveProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReserveProjection {
    /// Where the counter should be now.
    pub amount: u32,
    pub recorded_at_ms: i64,
    /// When it will be full, if it is not yet.
    pub full_at_ms: Option<i64>,
    /// Since when it has been full, if it is.
    pub full_since_ms: Option<i64>,
    /// Recharge lost to the cap since then.
    pub wasted: u32,
}
