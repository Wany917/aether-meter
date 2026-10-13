//! Bosses that come back: field bosses some time after each death, and
//! scheduled ones at fixed times on the server's clock. The catalog says
//! which; the player's state keeps the last kill of each, seen by the meter
//! or entered by hand, and the bosses the player follows.

use serde::{Deserialize, Serialize};

use crate::catalog::{Localized, localize};
use crate::error::RoutineError;
use crate::routine::{Reminder, Routine};
use crate::schedule::{DAY_MS, Weekday};
use crate::view::{BossRule, BossStatus, BossView};
use crate::zone::ServerZone;

const MINUTE_MS: i64 = 60_000;
/// How far back a kill entered by hand may go.
const MAX_KILL_AGE_MS: i64 = 7 * DAY_MS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BossDef {
    pub id: String,
    /// The NPC codes it spawns as: one per faction's version, or per variant.
    pub npc_codes: Vec<i32>,
    pub label: Localized,
    #[serde(default)]
    pub zone: Option<Localized>,
    #[serde(default)]
    pub note: Option<Localized>,
    pub respawn: Respawn,
    #[serde(default)]
    pub guide: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Respawn {
    /// Back this long after each death.
    AfterKill { minutes: u32 },
    /// At fixed times on the server's clock, up for a while each time.
    Schedule {
        zone: ServerZone,
        slots: Vec<Slot>,
        #[serde(default = "default_window")]
        window_minutes: u16,
    },
}

fn default_window() -> u16 {
    30
}

/// `{ "day": "friday", "at": "21:00" }`; no day means every day.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    #[serde(default)]
    pub day: Option<Weekday>,
    pub at: String,
}

impl Slot {
    /// Minutes after local midnight, if `at` is a valid `HH:MM`.
    pub fn minutes(&self) -> Option<i64> {
        let (h, m) = self.at.split_once(':')?;
        if h.len() != 2 || m.len() != 2 {
            return None;
        }
        let (h, m) = (h.parse::<i64>().ok()?, m.parse::<i64>().ok()?);
        ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 60 + m)
    }
}

impl BossDef {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.npc_codes.is_empty() || self.npc_codes.iter().any(|c| *c <= 0) {
            return Err("npcCodes needs at least one positive code".into());
        }
        match &self.respawn {
            Respawn::AfterKill { minutes } if !(1..=10_080).contains(minutes) => {
                Err("minutes must be 1 to 10080".into())
            }
            Respawn::Schedule {
                slots,
                window_minutes,
                ..
            } => {
                if slots.is_empty() || slots.len() > 48 {
                    Err("a schedule needs 1 to 48 slots".into())
                } else if slots.iter().any(|s| s.minutes().is_none()) {
                    Err("slot times are HH:MM".into())
                } else if !(1..=720).contains(window_minutes) {
                    Err("windowMinutes must be 1 to 720".into())
                } else {
                    Ok(())
                }
            }
            Respawn::AfterKill { .. } => Ok(()),
        }
    }

    /// Where the boss stands at `now`, given its last known death.
    pub fn status(&self, kill: Option<i64>, now: i64) -> BossStatus {
        match &self.respawn {
            Respawn::AfterKill { minutes } => match kill {
                None => BossStatus::Unknown,
                Some(kill) => {
                    let back = kill + i64::from(*minutes) * MINUTE_MS;
                    if back > now {
                        BossStatus::Respawning { at_ms: back }
                    } else {
                        BossStatus::Up { since_ms: back }
                    }
                }
            },
            Respawn::Schedule {
                zone,
                slots,
                window_minutes,
            } => {
                let times = occurrences(*zone, slots, now);
                let window = i64::from(*window_minutes) * MINUTE_MS;
                let last = times.iter().copied().filter(|t| *t <= now).max();
                match last {
                    Some(last) if now < last + window && kill.is_none_or(|k| k < last) => {
                        BossStatus::Up { since_ms: last }
                    }
                    _ => times
                        .into_iter()
                        .filter(|t| *t > now)
                        .min()
                        .map_or(BossStatus::Unknown, |at_ms| BossStatus::Scheduled { at_ms }),
                }
            }
        }
    }

    fn rule(&self, now: i64) -> BossRule {
        match &self.respawn {
            Respawn::AfterKill { minutes } => BossRule::AfterKill { minutes: *minutes },
            Respawn::Schedule { zone, slots, .. } => {
                let mut next_slots_ms: Vec<i64> = slots
                    .iter()
                    .filter_map(|slot| {
                        occurrences(*zone, std::slice::from_ref(slot), now)
                            .into_iter()
                            .filter(|t| *t > now)
                            .min()
                    })
                    .collect();
                next_slots_ms.sort_unstable();
                BossRule::Schedule {
                    next_slots_ms,
                    daily: slots.iter().all(|s| s.day.is_none()),
                }
            }
        }
    }
}

/// Every time a slot falls within eight days of `around`.
fn occurrences(zone: ServerZone, slots: &[Slot], around: i64) -> Vec<i64> {
    let today = zone.local_day(around);
    let mut times = Vec::new();
    for day in today - 8..=today + 8 {
        // 1970-01-01 was a Thursday (Monday = 0).
        let weekday = (day + 3).rem_euclid(7);
        for slot in slots {
            let Some(minutes) = slot.minutes() else {
                continue;
            };
            if slot.day.is_none_or(|d| d.index() == weekday) {
                times.push(zone.to_utc(day * DAY_MS + minutes * MINUTE_MS));
            }
        }
    }
    times
}

impl Routine {
    pub(crate) fn boss_views(&self, now: i64, language: &str) -> Vec<BossView> {
        self.catalog
            .bosses
            .iter()
            .map(|b| {
                let kill = self.state.bosses.kills.get(&b.id).copied();
                BossView {
                    id: b.id.clone(),
                    label: localize(&b.label, language),
                    zone: b.zone.as_ref().map(|z| localize(z, language)),
                    note: b.note.as_ref().map(|n| localize(n, language)),
                    guide: b.guide.clone(),
                    verified: b.verified,
                    favorite: self.state.bosses.favorites.contains(&b.id),
                    rule: b.rule(now),
                    status: b.status(kill, now),
                    last_kill_ms: kill,
                }
            })
            .collect()
    }

    /// A monster died within sight: the boss it was, if the catalog has it.
    pub fn record_kill(&mut self, npc_code: i32, at: i64) -> Option<String> {
        let boss = self
            .catalog
            .bosses
            .iter()
            .find(|b| b.npc_codes.contains(&npc_code))?;
        let kills = &mut self.state.bosses.kills;
        if kills.get(&boss.id).is_some_and(|k| *k >= at) {
            return None;
        }
        kills.insert(boss.id.clone(), at);
        Some(boss.id.clone())
    }

    /// A kill seen elsewhere (the in-game map, the guild), or `None` to forget it.
    pub fn set_boss_kill(
        &mut self,
        id: &str,
        at: Option<i64>,
        now: i64,
    ) -> Result<(), RoutineError> {
        self.boss_def(id)?;
        match at {
            None => {
                self.state.bosses.kills.remove(id);
            }
            Some(at) if at > now + MINUTE_MS || at < now - MAX_KILL_AGE_MS => {
                return Err(RoutineError::InvalidKill);
            }
            Some(at) => {
                self.state.bosses.kills.insert(id.to_string(), at);
            }
        }
        Ok(())
    }

    pub fn set_boss_favorite(&mut self, id: &str, on: bool) -> Result<(), RoutineError> {
        self.boss_def(id)?;
        let favorites = &mut self.state.bosses.favorites;
        favorites.retain(|f| f != id);
        if on {
            favorites.push(id.to_string());
        }
        Ok(())
    }

    pub fn set_boss_lead(&mut self, minutes: u8) {
        self.state.bosses.lead_minutes = minutes.clamp(1, 60);
    }

    fn boss_def(&self, id: &str) -> Result<&BossDef, RoutineError> {
        self.catalog
            .bosses
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| RoutineError::UnknownBoss(id.to_string()))
    }

    /// Followed bosses about to come back, keyed so each is sent once.
    pub(crate) fn boss_reminders(
        &self,
        views: &[BossView],
        now: i64,
        fr: bool,
    ) -> Vec<(String, Reminder)> {
        let lead = i64::from(self.state.bosses.lead_minutes) * MINUTE_MS;
        views
            .iter()
            .filter(|b| b.favorite)
            .filter_map(|b| {
                let at = match b.status {
                    BossStatus::Respawning { at_ms } | BossStatus::Scheduled { at_ms } => at_ms,
                    BossStatus::Up { .. } | BossStatus::Unknown => return None,
                };
                if at - now > lead {
                    return None;
                }
                let minutes = ((at - now) as f64 / MINUTE_MS as f64).ceil().max(1.0) as i64;
                let title = if fr {
                    format!("{} dans {minutes} min", b.label)
                } else {
                    format!("{} in {minutes} min", b.label)
                };
                let body = b.zone.clone().unwrap_or_else(|| {
                    if fr {
                        "Boss suivi".into()
                    } else {
                        "Followed boss".into()
                    }
                });
                Some((format!("boss:{}:{at}", b.id), Reminder { title, body }))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::schedule::HOUR_MS;
    use crate::state::UserState;

    /// 2026-10-14, a Wednesday, at `h`:00 UTC (summer time in Berlin).
    fn wed(h: i64) -> i64 {
        1_791_936_000_000 + h * HOUR_MS
    }

    fn after_kill(minutes: u32) -> BossDef {
        BossDef {
            id: "test".into(),
            npc_codes: vec![1],
            label: Localized::from([("en".into(), "Test".into())]),
            zone: None,
            note: None,
            respawn: Respawn::AfterKill { minutes },
            guide: None,
            sources: Vec::new(),
            verified: false,
        }
    }

    fn scheduled(slots: &[(Option<Weekday>, &str)]) -> BossDef {
        BossDef {
            respawn: Respawn::Schedule {
                zone: ServerZone::Europe,
                slots: slots
                    .iter()
                    .map(|(day, at)| Slot {
                        day: *day,
                        at: (*at).into(),
                    })
                    .collect(),
                window_minutes: 30,
            },
            ..after_kill(1)
        }
    }

    #[test]
    fn field_bosses_come_back_after_their_death() {
        let boss = after_kill(360);
        assert_eq!(boss.status(None, wed(10)), BossStatus::Unknown);
        assert_eq!(
            boss.status(Some(wed(8)), wed(10)),
            BossStatus::Respawning { at_ms: wed(14) }
        );
        assert_eq!(
            boss.status(Some(wed(8)), wed(15)),
            BossStatus::Up { since_ms: wed(14) }
        );
    }

    #[test]
    fn scheduled_bosses_follow_the_berlin_clock() {
        // Friday and Sunday 21:00 in Berlin: 19:00 UTC in summer.
        let nahma = scheduled(&[
            (Some(Weekday::Friday), "21:00"),
            (Some(Weekday::Sunday), "21:00"),
        ]);
        let friday = wed(0) + 2 * DAY_MS;
        assert_eq!(
            nahma.status(None, wed(12)),
            BossStatus::Scheduled {
                at_ms: friday + 19 * HOUR_MS
            }
        );
        // Up for half an hour, unless it was seen killed.
        let during = friday + 19 * HOUR_MS + 10 * MINUTE_MS;
        assert_eq!(
            nahma.status(None, during),
            BossStatus::Up {
                since_ms: friday + 19 * HOUR_MS
            }
        );
        assert_eq!(
            nahma.status(Some(during), during + MINUTE_MS),
            BossStatus::Scheduled {
                at_ms: friday + 2 * DAY_MS + 19 * HOUR_MS
            }
        );
        // After October 25, 21:00 in Berlin is 20:00 UTC.
        let next_friday = friday + 14 * DAY_MS;
        assert_eq!(
            nahma.status(None, next_friday),
            BossStatus::Scheduled {
                at_ms: next_friday + 20 * HOUR_MS
            }
        );
    }

    #[test]
    fn daily_slots_list_their_next_times_in_order() {
        let boss = scheduled(&[(None, "22:00"), (None, "13:00")]);
        let BossRule::Schedule {
            next_slots_ms,
            daily,
        } = boss.rule(wed(12))
        else {
            panic!("a schedule");
        };
        assert!(daily);
        assert_eq!(next_slots_ms, vec![wed(20), wed(11) + DAY_MS]);
    }

    #[test]
    fn kills_seen_by_the_meter_start_the_timer() {
        let mut r = Routine::new(Catalog::bundled(), UserState::default());
        // Silent Dartan, either faction's version.
        assert_eq!(
            r.record_kill(2_400_855, wed(8)).as_deref(),
            Some("silent-dartan")
        );
        assert_eq!(
            r.record_kill(2_101_120, wed(7)),
            None,
            "older than the one known"
        );
        assert_eq!(
            r.record_kill(999, wed(9)),
            None,
            "not a boss of the catalog"
        );
        let view = r.view(wed(9), "fr");
        let dartan = view
            .bosses
            .iter()
            .find(|b| b.id == "silent-dartan")
            .unwrap();
        assert_eq!(dartan.status, BossStatus::Respawning { at_ms: wed(14) });
        assert!(
            r.set_boss_kill("silent-dartan", Some(wed(9) + HOUR_MS), wed(9))
                .is_err(),
            "in the future"
        );
    }

    #[test]
    fn followed_bosses_warn_before_they_come_back() {
        let mut r = Routine::new(Catalog::bundled(), UserState::default());
        r.record_kill(2_400_855, wed(8));
        assert!(
            r.due_reminders(wed(14) - 10 * MINUTE_MS, "fr").is_empty(),
            "not followed"
        );
        r.set_boss_favorite("silent-dartan", true).unwrap();
        let due = r.due_reminders(wed(14) - 4 * MINUTE_MS, "fr");
        assert_eq!(due.len(), 1);
        assert!(due[0].title.ends_with("dans 4 min"), "{}", due[0].title);
        assert!(
            r.due_reminders(wed(14) - 3 * MINUTE_MS, "fr").is_empty(),
            "sent once"
        );
    }
}
