//! Your buffs and skill cooldowns, live.
//!
//! Fed from the parser's frame tap: every decoded frame, compressed bundles
//! included. Buff records are decoded by the parser's `abnormal` reader; the
//! cooldown record (`47 38`) is decoded here:
//!
//! ```text
//! <varint len> 47 38 <count u8> (<skill u32 LE> <remaining ms varint>)*
//! ```
//!
//! Cooldown layout from RATmeter (GPL-3.0), `SkillCdProcessor.cs`.

use std::collections::HashMap;

use a2tools::capture::abnormal::{self, Applied, Record};
use a2tools::capture::stream_processor::read_varint;

use crate::clock::now_ms;
use crate::gamedata::GameData;
use crate::icons;
use crate::model::{TimerKind, TimerLabel, TimerView, TrackerSnapshot};

const COOLDOWN: [u8; 2] = [0x47, 0x38];
/// Shorter cooldowns are global-cooldown noise.
const MIN_COOLDOWN_MS: i64 = 3_000;
/// Longer buffs (food, scrolls) are not combat timers.
const MAX_BUFF_MS: i64 = 15 * 60_000;
/// Weight of the newest sample in the server clock offset estimate.
const SKEW_SMOOTHING: f64 = 0.2;

#[derive(Default)]
pub(crate) struct Tracker {
    own_entity: Option<i32>,
    /// Instance id -> buff, for your character only.
    buffs: HashMap<i32, LiveBuff>,
    /// Skill code -> cooldown.
    cooldowns: HashMap<u32, Countdown>,
    /// Local receive time minus server time, in ms (network lag included,
    /// which is what makes a fresh buff read its full length).
    skew_ms: Option<f64>,
}

struct LiveBuff {
    abnormal: u32,
    skill: Option<u32>,
    timer: Countdown,
}

#[derive(Clone, Copy)]
struct Countdown {
    end_ms: i64,
    total_ms: i64,
}

impl Tracker {
    /// The local character's entity id, as the parser learned it.
    pub fn set_own_entity(&mut self, entity: Option<i32>) {
        if entity.is_some() && entity != self.own_entity {
            self.own_entity = entity;
            self.buffs.clear();
        }
    }

    pub fn clear(&mut self) {
        self.buffs.clear();
        self.cooldowns.clear();
    }

    /// One decoded frame: its length varint, 2-byte type, body.
    pub fn on_frame(&mut self, frame: &[u8]) {
        let len = read_varint(frame, 0);
        if len.length <= 0 {
            return;
        }
        let at = len.length as usize;
        if frame.get(at..at + 2) == Some(&COOLDOWN) {
            self.on_cooldowns(&frame[at + 2..]);
            return;
        }
        if let Some(record) = abnormal::parse(frame) {
            self.on_abnormal(record);
        }
    }

    fn on_cooldowns(&mut self, body: &[u8]) {
        let Some((&count, mut rest)) = body.split_first() else {
            return;
        };
        let now = now_ms();
        for _ in 0..count {
            let Some(skill) = rest
                .get(..4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            else {
                return;
            };
            let remaining = read_varint(rest, 4);
            if remaining.length <= 0 {
                return;
            }
            rest = &rest[4 + remaining.length as usize..];
            let ms = i64::from(remaining.value);
            if ms <= 0 {
                self.cooldowns.remove(&skill);
            } else {
                self.cooldowns.insert(
                    skill,
                    Countdown {
                        end_ms: now + ms,
                        total_ms: ms,
                    },
                );
            }
        }
    }

    fn on_abnormal(&mut self, record: Record) {
        match record {
            Record::Listed { entity, own, list } => {
                if own {
                    self.own_entity = Some(entity);
                }
                if Some(entity) == self.own_entity {
                    self.buffs.clear();
                    for applied in &list {
                        self.apply(applied, false);
                    }
                }
            }
            Record::Added(applied) => self.apply(&applied, true),
            Record::Changed(applied) => {
                let restarted = applied.restarted;
                self.apply(&applied, restarted);
            }
            Record::Removed { entity, list } => {
                if Some(entity) == self.own_entity {
                    for removal in list {
                        self.buffs.remove(&removal.instance);
                    }
                }
            }
            Record::Stats { .. } => {}
        }
    }

    fn apply(&mut self, applied: &Applied, fresh_start: bool) {
        if Some(applied.entity) != self.own_entity {
            return;
        }
        let (Some(length), Some(end)) = (applied.length_ms, applied.end_ms) else {
            // Endless (passives, toggles): not a timer.
            self.buffs.remove(&applied.instance);
            return;
        };
        if fresh_start {
            let sample = (now_ms() - (end - length)) as f64;
            self.skew_ms = Some(match self.skew_ms {
                Some(skew) => skew + SKEW_SMOOTHING * (sample - skew),
                None => sample,
            });
        }
        let skew = self.skew_ms.unwrap_or(0.0) as i64;
        self.buffs.insert(
            applied.instance,
            LiveBuff {
                abnormal: applied.abnormal,
                skill: applied.skill,
                timer: Countdown {
                    end_ms: end + skew,
                    total_ms: length,
                },
            },
        );
    }

    pub fn snapshot(&mut self, data: &GameData) -> TrackerSnapshot {
        let now = now_ms();
        self.buffs.retain(|_, b| b.timer.end_ms > now);
        self.cooldowns.retain(|_, c| c.end_ms > now);

        // One entry per abnormal, its stacks counted.
        let mut stacked: HashMap<u32, (Countdown, Option<u32>, u8)> = HashMap::new();
        for buff in self
            .buffs
            .values()
            .filter(|b| b.timer.total_ms <= MAX_BUFF_MS)
        {
            let entry = stacked
                .entry(buff.abnormal)
                .or_insert((buff.timer, buff.skill, 0));
            entry.2 = entry.2.saturating_add(1);
            if buff.timer.end_ms > entry.0.end_ms {
                entry.0 = buff.timer;
            }
        }
        let mut buffs: Vec<TimerView> = stacked
            .into_iter()
            .map(|(abnormal, (timer, skill, stacks))| {
                let name = data
                    .abnormal_name(abnormal)
                    .or_else(|| {
                        skill
                            .map(|s| data.skill_name(s as i32))
                            .filter(|n| !n.is_empty())
                    })
                    .unwrap_or_else(|| format!("#{abnormal}"));
                let icons = icons::abnormal_icons(
                    data.abnormal_icon(abnormal),
                    skill,
                    data.skill_icon_table(),
                );
                timer_view(TimerKind::Buff, abnormal, name, icons, timer, now, stacks)
            })
            .collect();

        let mut cooldowns: Vec<TimerView> = self
            .cooldowns
            .iter()
            .filter(|(_, c)| c.total_ms >= MIN_COOLDOWN_MS)
            .map(|(&skill, &timer)| {
                let code = skill as i32;
                let name = Some(data.skill_name(code))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("#{skill}"));
                let icons = icons::skill_icons(code, data.skill_icon_table());
                timer_view(TimerKind::Cooldown, skill, name, icons, timer, now, 1)
            })
            .collect();

        buffs.sort_by_key(|t| (t.remaining_ms, t.id));
        cooldowns.sort_by_key(|t| (t.remaining_ms, t.id));
        TrackerSnapshot { buffs, cooldowns }
    }
}

fn timer_view(
    kind: TimerKind,
    id: u32,
    name: String,
    icons: Vec<String>,
    timer: Countdown,
    now: i64,
    stacks: u8,
) -> TimerView {
    let remaining = (timer.end_ms - now).clamp(0, i64::from(u32::MAX));
    TimerView {
        kind,
        id,
        name,
        icons,
        remaining_ms: remaining as u32,
        total_ms: timer.total_ms.clamp(remaining, i64::from(u32::MAX)) as u32,
        stacks,
    }
}

/// Name and icons of a buff (abnormal id) or a cooldown (skill code).
pub(crate) fn label(kind: TimerKind, id: u32, data: &GameData) -> TimerLabel {
    let (name, icons) = match kind {
        TimerKind::Buff => (
            data.abnormal_name(id),
            icons::abnormal_icons(data.abnormal_icon(id), None, data.skill_icon_table()),
        ),
        TimerKind::Cooldown => {
            let code = id as i32;
            (
                Some(data.skill_name(code)).filter(|n| !n.is_empty()),
                icons::skill_icons(code, data.skill_icon_table()),
            )
        }
    };
    TimerLabel {
        kind,
        id,
        name: name.unwrap_or_else(|| format!("#{id}")),
        icons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_records_start_and_clear_timers() {
        let mut tracker = Tracker::default();
        // len, 47 38, count 2, skill 11010000 for 12 000 ms, skill 17040000 cleared.
        let mut frame = vec![0x00, 0x47, 0x38, 0x02];
        frame.extend_from_slice(&11_010_000u32.to_le_bytes());
        frame.extend_from_slice(&[0xE0, 0x5D]); // 12000 as a varint
        frame.extend_from_slice(&17_040_000u32.to_le_bytes());
        frame.push(0x00);
        frame[0] = frame.len() as u8;
        tracker.cooldowns.insert(
            17_040_000,
            Countdown {
                end_ms: now_ms() + 5_000,
                total_ms: 5_000,
            },
        );

        tracker.on_frame(&frame);

        assert!(!tracker.cooldowns.contains_key(&17_040_000));
        let cd = tracker.cooldowns[&11_010_000];
        assert_eq!(cd.total_ms, 12_000);
        assert!((cd.end_ms - now_ms() - 12_000).abs() < 1_000);
    }
}
