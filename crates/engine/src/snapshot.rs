//! Parser structs -> UI model.

use std::collections::HashMap;

use a2tools::entity::details_context::{
    DetailSkillEntry, DetailsActorSummary, TargetDetailsResponse,
};
use a2tools::entity::dps_data::DpsData;
use a2tools::entity::fight_record::FightRecord;

use crate::gamedata::GameData;
use crate::history::IndexEntry;
use crate::icons;
use crate::model::{
    ClassId, EngineStatus, FightSource, FightSummary, FightView, MeterSnapshot, PlayerDetails,
    PlayerRow, SkillRow, TargetView,
};

/// `dummy`: the target is a training dummy.
pub(crate) fn meter(
    dps: &DpsData,
    status: EngineStatus,
    ping_ms: Option<u32>,
    dummy: bool,
    data: &GameData,
) -> MeterSnapshot {
    let local = dps.local_player_id.and_then(|id| i32::try_from(id).ok());
    let total: f64 = dps.map.values().map(|p| p.amount.max(0.0)).sum();

    let mut players: Vec<PlayerRow> = dps
        .map
        .iter()
        .filter(|&(&id, _)| id > 0)
        .map(|(&id, p)| PlayerRow {
            actor_id: id,
            name: display_name(&p.nickname, id),
            class: ClassId::from_parser_name(&p.job),
            damage: p.amount.max(0.0) as u64,
            dps: p.dps.max(0.0),
            share: ratio(p.amount, total),
            is_local: Some(id) == local,
            combat_power: p.combat_power.max(0) as u64,
        })
        .collect();
    players.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.actor_id.cmp(&b.actor_id)));

    let target = (dps.target_id != 0).then(|| {
        let max_hp = dps.target_max_hp.max(0);
        let hp_ratio = (max_hp > 0).then(|| {
            let current = if dps.target_current_hp >= 0 {
                dps.target_current_hp
            } else {
                (max_hp - dps.target_total_damage).max(0)
            };
            (current as f64 / max_hp as f64).clamp(0.0, 1.0)
        });
        TargetView {
            id: dps.target_id,
            name: data.target_label(dps.target_mob_code, &dps.target_name),
            is_boss: dps.target_is_boss,
            is_dummy: dummy,
            max_hp: max_hp as u64,
            hp_ratio,
        }
    });

    let battle_time_ms = dps.battle_time.max(0) as u64;
    MeterSnapshot {
        status,
        target,
        battle_time_ms,
        total_damage: total as u64,
        party_dps: per_second(total, battle_time_ms),
        players,
        ping_ms,
        dungeon: data.dungeon_label(dps.dungeon_id),
        recording: false,
    }
}

pub(crate) struct FightHeader {
    pub source: FightSource,
    pub target_name: String,
    pub dungeon: Option<String>,
    pub started_at_ms: i64,
    pub local_actor: Option<i32>,
}

pub(crate) fn fight(
    header: FightHeader,
    details: &TargetDetailsResponse,
    actors: &[DetailsActorSummary],
    data: &GameData,
) -> FightView {
    let duration_ms = details.battle_time.max(0) as u64;
    let actor_info: HashMap<i32, &DetailsActorSummary> =
        actors.iter().map(|a| (a.actor_id, a)).collect();

    let mut by_actor: HashMap<i32, Vec<&DetailSkillEntry>> = HashMap::new();
    for entry in &details.skills {
        by_actor.entry(entry.actor_id).or_default().push(entry);
    }
    let mut healing: HashMap<i32, u64> = HashMap::new();
    for entry in &details.heal_skills {
        *healing.entry(entry.actor_id).or_default() += entry.dmg.max(0) as u64;
    }

    let fight_total: u64 = details.skills.iter().map(|s| s.dmg.max(0) as u64).sum();
    let mut players: Vec<PlayerDetails> = by_actor
        .into_iter()
        .filter(|(id, _)| *id > 0)
        .map(|(id, entries)| {
            let info = actor_info.get(&id);
            let damage: u64 = entries.iter().map(|e| e.dmg.max(0) as u64).sum();
            let direct: Vec<&&DetailSkillEntry> = entries.iter().filter(|e| !e.is_dot).collect();
            let hits: u32 = direct.iter().map(|e| e.time.max(0) as u32).sum();
            let rate = |f: fn(&DetailSkillEntry) -> i32| {
                ratio(
                    direct.iter().map(|e| f(e).max(0) as f64).sum(),
                    f64::from(hits),
                )
            };
            let class = info
                .map(|a| ClassId::from_prefix(a.job_id))
                .filter(|c| *c != ClassId::Unknown)
                .or_else(|| {
                    entries
                        .iter()
                        .map(|e| ClassId::from_parser_name(&e.job))
                        .find(|c| *c != ClassId::Unknown)
                })
                .unwrap_or(ClassId::Unknown);

            let mut skills: Vec<SkillRow> =
                entries.iter().map(|e| skill_row(e, damage, data)).collect();
            skills.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.code.cmp(&b.code)));

            PlayerDetails {
                actor_id: id,
                name: display_name(info.map_or("", |a| a.nickname.as_str()), id),
                class,
                is_local: Some(id) == header.local_actor,
                damage,
                dps: per_second(damage as f64, duration_ms),
                share: ratio(damage as f64, fight_total as f64),
                hits,
                crit_rate: rate(|e| e.crit),
                back_rate: rate(|e| e.back),
                perfect_rate: rate(|e| e.perfect),
                healing: healing.get(&id).copied().unwrap_or(0),
                combat_power: info.map_or(0, |a| a.combat_power.max(0) as u64),
                skills,
            }
        })
        .filter(|p| p.damage > 0 || p.healing > 0)
        .collect();
    players.sort_by(|a, b| b.damage.cmp(&a.damage).then(a.actor_id.cmp(&b.actor_id)));

    FightView {
        source: header.source,
        target_name: header.target_name,
        dungeon: header.dungeon,
        started_at_ms: header.started_at_ms.max(0) as u64,
        duration_ms,
        total_damage: fight_total,
        max_hp: details.max_hp.max(0) as u64,
        players,
    }
}

fn skill_row(e: &DetailSkillEntry, actor_damage: u64, data: &GameData) -> SkillRow {
    let damage = e.dmg.max(0) as u64;
    let hits = e.time.max(0) as u32;
    let per_hit = |n: i32| ratio(f64::from(n.max(0)), f64::from(hits));
    // Re-resolved so saved fights follow the current language.
    let base = Some(data.skill_name(e.code))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| e.name.clone());
    let name = if base.is_empty() {
        format!("#{}", e.code)
    } else if e.is_dot && !base.ends_with("DOT") {
        format!("{base} · DoT")
    } else {
        base
    };
    SkillRow {
        code: e.code,
        name,
        icons: icons::skill_icons(e.code, data.skill_icon_table()),
        is_dot: e.is_dot,
        damage,
        share: ratio(damage as f64, actor_damage as f64),
        hits,
        crit_rate: per_hit(e.crit),
        back_rate: per_hit(e.back),
        perfect_rate: per_hit(e.perfect),
        double_rate: per_hit(e.double),
        parry_rate: per_hit(e.parry),
        min_hit: e.min_dmg.max(0) as u64,
        max_hit: e.max_dmg.max(0) as u64,
        avg_hit: ratio(damage as f64, f64::from(hits)),
    }
}

pub(crate) fn saved_fight(record: &FightRecord, data: &GameData) -> FightView {
    let header = FightHeader {
        source: FightSource::Saved {
            id: record.id.clone(),
        },
        target_name: target_name(&record.boss_name, record.mob_code, data),
        dungeon: data.dungeon_label(record.dungeon_id),
        started_at_ms: record.start_time_ms,
        local_actor: None,
    };
    fight(header, &record.details, &record.actors, data)
}

pub(crate) fn summary(entry: &IndexEntry, data: &GameData) -> FightSummary {
    FightSummary {
        id: entry.id.clone(),
        target_name: target_name(&entry.boss_name, entry.mob_code, data),
        dungeon: data.dungeon_label(entry.dungeon_id),
        started_at_ms: entry.started_at_ms.max(0) as u64,
        duration_ms: entry.duration_ms.max(0) as u64,
        total_damage: entry.total_damage.max(0) as u64,
        classes: entry
            .job_ids
            .iter()
            .map(|&p| ClassId::from_prefix(p))
            .collect(),
        is_training: entry.is_train,
    }
}

fn target_name(saved: &str, mob_code: i32, data: &GameData) -> String {
    data.target_label(mob_code, saved)
}

fn display_name(nickname: &str, id: i32) -> String {
    if nickname.trim().is_empty() {
        format!("#{id}")
    } else {
        nickname.to_string()
    }
}

fn ratio(part: f64, whole: f64) -> f64 {
    if whole > 0.0 {
        (part / whole).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn per_second(amount: f64, duration_ms: u64) -> f64 {
    if duration_ms == 0 {
        0.0
    } else {
        amount / (duration_ms.max(1000) as f64 / 1000.0)
    }
}
