//! A synthetic party fighting a synthetic boss, on a loop. For working on the
//! overlay without the game (or on a machine that cannot run it).

use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

use super::Feed;
use crate::clock::now_ms;
use crate::gamedata::GameData;
use crate::icons;
use crate::model::{
    ClassId, EngineStatus, FightSource, FightView, MeterSnapshot, PlayerDetails, PlayerRow,
    SkillRow, TargetMode, TargetView, TimerKind, TimerView, TrackerSnapshot,
};

const BOSS_NAME: &str = "Gardien d'Aether";
const BOSS_HP: f64 = 52_000_000.0;
const PAUSE_AFTER_KILL_S: f64 = 8.0;

struct DemoPlayer {
    id: i32,
    name: &'static str,
    class: ClassId,
    base_dps: f64,
    phase: f64,
    local: bool,
}

const PARTY: [DemoPlayer; 6] = [
    DemoPlayer {
        id: 101,
        name: "Daeva",
        class: ClassId::Gladiator,
        base_dps: 78_000.0,
        phase: 0.0,
        local: true,
    },
    DemoPlayer {
        id: 102,
        name: "Lyria",
        class: ClassId::Sorcerer,
        base_dps: 86_000.0,
        phase: 1.3,
        local: false,
    },
    DemoPlayer {
        id: 103,
        name: "Kaelen",
        class: ClassId::Ranger,
        base_dps: 71_000.0,
        phase: 2.1,
        local: false,
    },
    DemoPlayer {
        id: 104,
        name: "Morwen",
        class: ClassId::Templar,
        base_dps: 42_000.0,
        phase: 0.7,
        local: false,
    },
    DemoPlayer {
        id: 105,
        name: "Sylas",
        class: ClassId::Brawler,
        base_dps: 66_000.0,
        phase: 2.9,
        local: false,
    },
    DemoPlayer {
        id: 106,
        name: "Oriane",
        class: ClassId::Cleric,
        base_dps: 21_000.0,
        phase: 1.8,
        local: false,
    },
];

/// Relative weight of each of a player's eight skills.
const SKILL_WEIGHTS: [f64; 8] = [0.24, 0.19, 0.15, 0.12, 0.1, 0.09, 0.06, 0.05];

pub(crate) struct DemoFeed {
    data: Arc<GameData>,
    started: Mutex<Instant>,
}

impl DemoFeed {
    pub fn new(data: Arc<GameData>) -> Self {
        Self {
            data,
            started: Mutex::new(Instant::now()),
        }
    }

    /// Seconds into the current fight, looping after each kill.
    fn fight_time(&self) -> f64 {
        let elapsed = self.started.lock().elapsed().as_secs_f64();
        let kill = self.kill_time();
        elapsed % (kill + PAUSE_AFTER_KILL_S)
    }

    fn kill_time(&self) -> f64 {
        BOSS_HP / PARTY.iter().map(|p| p.base_dps).sum::<f64>()
    }

    fn damage(p: &DemoPlayer, t: f64) -> f64 {
        // Monotonic with a little swing: d/dt = base * (1 + 0.6 cos(..)) > 0.
        (p.base_dps * (t + 3.0 * ((t / 5.0 + p.phase).sin() - p.phase.sin()))).max(0.0)
    }

    fn skills(&self, p: &DemoPlayer, damage: f64) -> Vec<SkillRow> {
        let prefix = p.class.prefix().unwrap_or(11);
        SKILL_WEIGHTS
            .iter()
            .enumerate()
            .map(|(i, weight)| {
                let code = prefix * 1_000_000 + (i as i32 + 1) * 10_000;
                let skill_damage = damage * weight;
                let hits = (skill_damage / (p.base_dps * 0.9)).ceil().max(1.0) as u32;
                let name = Some(self.data.skill_name(code))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("Compétence {}", i + 1));
                SkillRow {
                    code,
                    name,
                    icons: icons::skill_icons(code, self.data.skill_icon_table()),
                    is_dot: false,
                    damage: skill_damage as u64,
                    share: *weight,
                    hits,
                    crit_rate: 0.32 + 0.03 * i as f64,
                    back_rate: 0.18,
                    perfect_rate: 0.07,
                    double_rate: 0.04,
                    parry_rate: 0.02,
                    min_hit: (skill_damage / f64::from(hits) * 0.6) as u64,
                    max_hit: (skill_damage / f64::from(hits) * 1.7) as u64,
                    avg_hit: skill_damage / f64::from(hits),
                }
            })
            .collect()
    }
}

impl Feed for DemoFeed {
    fn meter(&self) -> MeterSnapshot {
        let kill = self.kill_time();
        let t = self.fight_time().min(kill);
        let damages: Vec<f64> = PARTY.iter().map(|p| Self::damage(p, t)).collect();
        let total: f64 = damages.iter().sum();
        let mut players: Vec<PlayerRow> = PARTY
            .iter()
            .zip(&damages)
            .map(|(p, &d)| PlayerRow {
                actor_id: p.id,
                name: p.name.to_string(),
                class: p.class,
                damage: d as u64,
                dps: if t > 0.0 { d / t.max(1.0) } else { 0.0 },
                share: if total > 0.0 { d / total } else { 0.0 },
                is_local: p.local,
                combat_power: 78_400 + (p.id as u64 % 7) * 2_630,
            })
            .collect();
        players.sort_by_key(|p| std::cmp::Reverse(p.damage));

        MeterSnapshot {
            status: EngineStatus::Demo,
            target: Some(TargetView {
                id: 9_000,
                name: BOSS_NAME.to_string(),
                is_boss: true,
                max_hp: BOSS_HP as u64,
                hp_ratio: Some((1.0 - total / BOSS_HP).clamp(0.0, 1.0)),
            }),
            battle_time_ms: (t * 1000.0) as u64,
            total_damage: total as u64,
            party_dps: if t > 0.0 { total / t.max(1.0) } else { 0.0 },
            players,
            ping_ms: Some(38 + (t as u32 % 9)),
            dungeon: Some("Temple du Feu · Difficile".to_string()),
            recording: false,
        }
    }

    fn live_fight(&self) -> Option<FightView> {
        let t = self.fight_time().min(self.kill_time());
        let total: f64 = PARTY.iter().map(|p| Self::damage(p, t)).sum();
        let mut players: Vec<PlayerDetails> = PARTY
            .iter()
            .map(|p| {
                let damage = Self::damage(p, t);
                let skills = self.skills(p, damage);
                let hits = skills.iter().map(|s| s.hits).sum();
                PlayerDetails {
                    actor_id: p.id,
                    name: p.name.to_string(),
                    class: p.class,
                    is_local: p.local,
                    damage: damage as u64,
                    dps: damage / t.max(1.0),
                    share: if total > 0.0 { damage / total } else { 0.0 },
                    hits,
                    crit_rate: 0.38,
                    back_rate: 0.18,
                    perfect_rate: 0.07,
                    healing: if p.class == ClassId::Cleric {
                        (t * 24_000.0) as u64
                    } else {
                        0
                    },
                    combat_power: 78_400 + (p.id as u64 % 7) * 2_630,
                    skills,
                }
            })
            .collect();
        players.sort_by_key(|p| std::cmp::Reverse(p.damage));
        Some(FightView {
            source: FightSource::Live,
            target_name: BOSS_NAME.to_string(),
            dungeon: Some("Temple du Feu · Difficile".to_string()),
            started_at_ms: (now_ms() - (t * 1000.0) as i64).max(0) as u64,
            duration_ms: (t * 1000.0) as u64,
            total_damage: total as u64,
            max_hp: BOSS_HP as u64,
            players,
        })
    }

    fn tracker(&self) -> TrackerSnapshot {
        let t_ms = (self.started.lock().elapsed().as_millis() % u128::from(u32::MAX)) as u32;
        let table = self.data.skill_icon_table();
        let timer = |kind, id: u32, code: i32, total_ms: u32, offset_ms: u32, stacks| {
            let name = Some(self.data.skill_name(code))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| format!("#{code}"));
            TimerView {
                kind,
                id,
                name,
                icons: icons::skill_icons(code, table),
                remaining_ms: total_ms - (t_ms + offset_ms) % total_ms,
                total_ms,
                stacks,
            }
        };
        let mut buffs = vec![
            timer(TimerKind::Buff, 1, 11_050_000, 20_000, 0, 1),
            timer(TimerKind::Buff, 2, 11_090_000, 12_000, 4_000, 3),
            timer(TimerKind::Buff, 3, 18_030_000, 30_000, 9_000, 1),
        ];
        let mut cooldowns = vec![
            timer(TimerKind::Cooldown, 11_020_000, 11_020_000, 8_000, 0, 1),
            timer(
                TimerKind::Cooldown,
                11_040_000,
                11_040_000,
                15_000,
                3_000,
                1,
            ),
            timer(
                TimerKind::Cooldown,
                11_060_000,
                11_060_000,
                30_000,
                11_000,
                1,
            ),
            timer(
                TimerKind::Cooldown,
                11_080_000,
                11_080_000,
                45_000,
                20_000,
                1,
            ),
        ];
        buffs.sort_by_key(|t| t.remaining_ms);
        cooldowns.sort_by_key(|t| t.remaining_ms);
        TrackerSnapshot { buffs, cooldowns }
    }

    fn reset(&self) {
        *self.started.lock() = Instant::now();
    }

    fn set_target_mode(&self, _mode: TargetMode) {}

    fn save_fights(&self, _force: bool) {}

    fn take_kills(&self) -> Vec<crate::kills::Kill> {
        Vec::new()
    }

    fn take_pets(&self) -> Option<crate::pets::PetBatch> {
        None
    }
}
