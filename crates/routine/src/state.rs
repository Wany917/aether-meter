//! What the player did: per-task progress, reserve readings, their own tasks.
//! Kept apart from the catalog, which can be replaced by a newer revision.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::catalog::TaskGroup;
use crate::schedule::Cadence;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserState {
    pub membership: bool,
    /// Task id -> progress in the period it was made in.
    pub progress: BTreeMap<String, Progress>,
    /// Reserve id -> last value the player entered.
    pub readings: BTreeMap<String, Reading>,
    pub custom_tasks: Vec<CustomTask>,
    /// Catalog task and reserve ids the player hid.
    pub hidden: Vec<String>,
    pub reminders: ReminderSettings,
    /// Fetch newer catalog revisions from the project's repository.
    pub community_updates: bool,
    /// Reminders already sent, so each is sent once.
    pub notified: Vec<String>,
    pub bosses: BossState,
}

impl Default for UserState {
    fn default() -> Self {
        Self {
            membership: false,
            progress: BTreeMap::new(),
            readings: BTreeMap::new(),
            custom_tasks: Vec::new(),
            hidden: Vec::new(),
            reminders: ReminderSettings::default(),
            community_updates: true,
            notified: Vec::new(),
            bosses: BossState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BossState {
    /// Boss ids the player follows: shown on the overlay, announced before they return.
    pub favorites: Vec<String>,
    /// Boss id -> last death seen or entered, epoch ms.
    pub kills: BTreeMap<String, i64>,
    /// Announce a followed boss this long before it returns.
    pub lead_minutes: u8,
}

impl Default for BossState {
    fn default() -> Self {
        Self {
            favorites: Vec::new(),
            kills: BTreeMap::new(),
            lead_minutes: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub count: u16,
    pub period_start: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub amount: u32,
    pub at: i64,
    /// Read off the game's own records, not entered by the player.
    #[serde(default)]
    pub from_game: bool,
    /// What refill items added beside the counter, outside its cap (Odyle's
    /// charged energy); only the game says.
    #[serde(default)]
    pub refills: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTask {
    pub id: String,
    pub label: String,
    pub group: TaskGroup,
    pub cadence: Cadence,
    pub target: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReminderSettings {
    pub enabled: bool,
    /// Warn this long before a reset that would discard unfinished tasks.
    pub before_reset_hours: u8,
    /// Warn this long before a reserve reaches its cap.
    pub before_cap_hours: u8,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            before_reset_hours: 2,
            before_cap_hours: 6,
        }
    }
}
