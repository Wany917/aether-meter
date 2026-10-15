//! The UI contract: everything the overlay receives from the engine.
//!
//! These types are deliberately independent from the parser's own structs, so a
//! parser bump never changes what the frontend sees. Counts and damage are
//! `u64` on this side and exported as plain `number` to TypeScript (exact up to
//! 2^53, far beyond any fight). Every `f64` here is finite by construction (see
//! `snapshot::ratio`), so it is exported as `number` rather than specta's
//! default `number | null`.

use a2tools::entity::job_class::JobClass;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ClassId {
    Gladiator,
    Templar,
    Ranger,
    Assassin,
    Sorcerer,
    Elementalist,
    Cleric,
    Chanter,
    Brawler,
    Unknown,
}

impl ClassId {
    pub fn from_job(job: JobClass) -> Self {
        match job {
            JobClass::Gladiator => Self::Gladiator,
            JobClass::Templar => Self::Templar,
            JobClass::Ranger => Self::Ranger,
            JobClass::Assassin => Self::Assassin,
            JobClass::Sorcerer => Self::Sorcerer,
            JobClass::Elementalist => Self::Elementalist,
            JobClass::Cleric => Self::Cleric,
            JobClass::Chanter => Self::Chanter,
            JobClass::Fighter => Self::Brawler,
        }
    }

    /// The parser names classes by their Korean name (`JobClass::class_name`).
    pub fn from_parser_name(name: &str) -> Self {
        match name {
            "검성" => Self::Gladiator,
            "수호성" => Self::Templar,
            "궁성" => Self::Ranger,
            "살성" => Self::Assassin,
            "마도성" => Self::Sorcerer,
            "정령성" => Self::Elementalist,
            "치유성" => Self::Cleric,
            "호법성" => Self::Chanter,
            "권성" => Self::Brawler,
            _ => Self::Unknown,
        }
    }

    /// The two-digit class prefix of 8-digit skill codes (11 = Gladiator).
    pub fn from_prefix(prefix: i32) -> Self {
        match prefix {
            11 => Self::Gladiator,
            12 => Self::Templar,
            13 => Self::Assassin,
            14 => Self::Ranger,
            15 => Self::Sorcerer,
            16 => Self::Elementalist,
            17 => Self::Cleric,
            18 => Self::Chanter,
            19 => Self::Brawler,
            _ => Self::Unknown,
        }
    }

    pub fn prefix(self) -> Option<i32> {
        match self {
            Self::Gladiator => Some(11),
            Self::Templar => Some(12),
            Self::Assassin => Some(13),
            Self::Ranger => Some(14),
            Self::Sorcerer => Some(15),
            Self::Elementalist => Some(16),
            Self::Cleric => Some(17),
            Self::Chanter => Some(18),
            Self::Brawler => Some(19),
            Self::Unknown => None,
        }
    }
}

/// What the capture is doing, for the meter's status line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EngineStatus {
    /// The packet capture library (Npcap on Windows) could not be loaded.
    #[serde(rename_all = "camelCase")]
    CaptureUnavailable {
        reason: String,
    },
    /// Opening the capture library and the network adapters.
    StartingCapture,
    /// Capturing, but no game connection identified yet.
    WaitingForGame,
    /// Locked on the game's connection.
    #[serde(rename_all = "camelCase")]
    Live {
        server_port: u16,
    },
    #[serde(rename_all = "camelCase")]
    Replay {
        file: String,
        finished: bool,
    },
    Demo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TargetView {
    pub id: i32,
    pub name: String,
    pub is_boss: bool,
    /// A training dummy, even one whose spawn the meter never saw (it is
    /// then known by its HP, which resets): it has no name from the game.
    pub is_dummy: bool,
    /// 0 when the game never told us.
    pub max_hp: u64,
    /// Remaining HP as a fraction of max, when max is known.
    #[cfg_attr(feature = "specta", specta(type = Option<specta_typescript::Number>))]
    pub hp_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlayerRow {
    pub actor_id: i32,
    pub name: String,
    pub class: ClassId,
    pub damage: u64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub dps: f64,
    /// Fraction of the party's damage on the target, 0..1.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub share: f64,
    pub is_local: bool,
    /// From the party roster; 0 for players outside your party.
    pub combat_power: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MeterSnapshot {
    pub status: EngineStatus,
    pub target: Option<TargetView>,
    pub battle_time_ms: u64,
    pub total_damage: u64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub party_dps: f64,
    /// Sorted by damage, highest first.
    pub players: Vec<PlayerRow>,
    pub ping_ms: Option<u32>,
    pub dungeon: Option<String>,
    /// A recording for decoding the game is running (see `recorder.rs`).
    pub recording: bool,
}

impl MeterSnapshot {
    pub fn empty(status: EngineStatus) -> Self {
        Self {
            status,
            target: None,
            battle_time_ms: 0,
            total_damage: 0,
            party_dps: 0.0,
            players: Vec::new(),
            ping_ms: None,
            dungeon: None,
            recording: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SkillRow {
    pub code: i32,
    pub name: String,
    /// CDN icon URLs to try in order. Empty for basic attacks.
    pub icons: Vec<String>,
    pub is_dot: bool,
    pub damage: u64,
    /// Fraction of this player's damage, 0..1.
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub share: f64,
    pub hits: u32,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub crit_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub back_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub perfect_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub double_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub parry_rate: f64,
    pub min_hit: u64,
    pub max_hit: u64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub avg_hit: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetails {
    pub actor_id: i32,
    pub name: String,
    pub class: ClassId,
    pub is_local: bool,
    pub damage: u64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub dps: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub share: f64,
    pub hits: u32,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub crit_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub back_rate: f64,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Number))]
    pub perfect_rate: f64,
    pub healing: u64,
    pub combat_power: u64,
    /// Sorted by damage, highest first.
    pub skills: Vec<SkillRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FightSource {
    Live,
    Saved { id: String },
}

/// One fight with every player's breakdown: the live one or a saved one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FightView {
    pub source: FightSource,
    pub target_name: String,
    pub dungeon: Option<String>,
    /// Epoch ms; 0 when unknown.
    pub started_at_ms: u64,
    pub duration_ms: u64,
    pub total_damage: u64,
    pub max_hp: u64,
    /// Sorted by damage, highest first.
    pub players: Vec<PlayerDetails>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct FightSummary {
    pub id: String,
    pub target_name: String,
    pub dungeon: Option<String>,
    pub started_at_ms: u64,
    pub duration_ms: u64,
    pub total_damage: u64,
    pub classes: Vec<ClassId>,
    /// A training dummy rather than a boss.
    pub is_training: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TimerKind {
    Buff,
    Cooldown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TimerView {
    pub kind: TimerKind,
    /// Abnormal id for a buff, skill code for a cooldown.
    pub id: u32,
    pub name: String,
    pub icons: Vec<String>,
    pub remaining_ms: u32,
    pub total_ms: u32,
    pub stacks: u8,
}

/// A timer's name and icon, without its countdown (for lists of hidden timers).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TimerLabel {
    pub kind: TimerKind,
    pub id: u32,
    pub name: String,
    pub icons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TrackerSnapshot {
    /// Timed buffs on your character, soonest to expire first.
    pub buffs: Vec<TimerView>,
    /// Your skills on cooldown, soonest ready first.
    pub cooldowns: Vec<TimerView>,
}

/// How the meter picks the target whose fight it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TargetMode {
    /// The boss fight while it is going on, else your own last target.
    Auto,
    Boss,
    MostDamage,
    MostRecent,
    LastHitByMe,
    AllTargets,
}

impl TargetMode {
    /// The parser's own id for the mode (`Auto` runs the boss mode first).
    pub fn parser_id(self) -> &'static str {
        match self {
            Self::Auto | Self::Boss => "bossTargets",
            Self::MostDamage => "mostDamage",
            Self::MostRecent => "mostRecent",
            Self::LastHitByMe => "lastHitByMe",
            Self::AllTargets => "allTargets",
        }
    }
}
