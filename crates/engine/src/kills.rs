//! Monsters killed within sight, for the boss respawn timers.
//!
//! Fed from the parser's frame tap. The death record, as the parser reads it
//! (`StreamProcessor::parse_death_packet`):
//!
//! ```text
//! <varint len> 42 36 <entity varint> <varint, always 0> <flag varint>
//! ```
//!
//! Flag 3 is a death in combat (1 is an entity loaded dead, 7 one leaving the
//! world). The record names the entity, not the monster: its NPC code comes
//! from the parser's spawn table when the kills are taken.

use a2tools::capture::stream_processor::read_varint;
use a2tools::combat::data_storage::DataStorage;

use crate::clock::now_ms;

const DEATH: [u8; 2] = [0x42, 0x36];
const COMBAT_DEATH: i32 = 3;
/// Kills nobody took (the app is busy elsewhere) are dropped past this.
const MAX_PENDING: usize = 256;

/// A monster that died in combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kill {
    pub npc_code: i32,
    /// Local time of the death record, epoch ms.
    pub at_ms: i64,
}

#[derive(Default)]
pub(crate) struct KillLog {
    /// Entity id and time of each death not taken yet.
    pending: Vec<(i32, i64)>,
}

impl KillLog {
    pub fn on_frame(&mut self, frame: &[u8]) {
        if let Some(entity) = combat_death(frame) {
            if self.pending.len() >= MAX_PENDING {
                self.pending.remove(0);
            }
            self.pending.push((entity, now_ms()));
        }
    }

    /// The deaths since the last call, with the code each entity spawned as.
    pub fn take(&mut self, storage: &DataStorage) -> Vec<Kill> {
        self.pending
            .drain(..)
            .filter_map(|(entity, at_ms)| {
                storage
                    .mob_code(entity)
                    .map(|npc_code| Kill { npc_code, at_ms })
            })
            .collect()
    }
}

fn combat_death(frame: &[u8]) -> Option<i32> {
    let varint = |at: usize| {
        let v = read_varint(frame, at);
        (v.length > 0).then(|| (v.value, at + v.length as usize))
    };
    let (_, at) = varint(0)?;
    if frame.get(at..at + 2) != Some(&DEATH) {
        return None;
    }
    let (entity, at) = varint(at + 2)?;
    let (_, at) = varint(at)?;
    let (flag, _) = varint(at)?;
    (flag == COMBAT_DEATH).then_some(entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_combat_deaths_count() {
        // Entity 25804 (cc c9 01): a death in combat, then the same entity leaving.
        assert_eq!(
            combat_death(&[0x0B, 0x42, 0x36, 0xCC, 0xC9, 0x01, 0x00, 0x03]),
            Some(25804)
        );
        assert_eq!(
            combat_death(&[0x0B, 0x42, 0x36, 0xCC, 0xC9, 0x01, 0x00, 0x07]),
            None
        );
        assert_eq!(
            combat_death(&[0x0B, 0x41, 0x36, 0xCC, 0xC9, 0x01, 0x00, 0x03]),
            None
        );
        assert_eq!(combat_death(&[0x0B, 0x42, 0x36, 0xCC]), None, "truncated");
    }
}
