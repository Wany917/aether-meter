//! Your pet collection as the game reports it, for the pet soul tracker.
//!
//! Three records, laid out as A2Tools reads them for its pet soul tracker
//! (decoder tests in the GPL-3.0 meter's `plugins/decoder.rs`):
//!
//! ```text
//! 00 90  at login, every pet:  <skip 4> <n varint> (<pet u32> <skip 4> <level u32>)*
//!                                       <n varint> (<pet u32> <souls u32>)*
//! 0D 90  a soul picked up:     <skip 1> <pet u32> <amount u32>
//! 14 90  a level reached:      <skip 2> <pet u32> <level u32>
//! ```
//!
//! A collection is shared by every character on a server, so records are
//! handed over with the server they belong to. The login list comes before
//! the record that names your server: it waits a moment for it.

use std::collections::VecDeque;

use a2tools::capture::stream_processor::read_varint;
use a2tools::combat::data_storage::DataStorage;

use crate::clock::now_ms;

const LIST: [u8; 2] = [0x00, 0x90];
const SOUL: [u8; 2] = [0x0D, 0x90];
const LEVEL: [u8; 2] = [0x14, 0x90];
/// How long a login list waits for the record naming the server.
const LIST_WAIT_MS: i64 = 3_000;
/// Records nobody took are dropped past this.
const MAX_PENDING: usize = 64;
/// Bounds that keep a misread frame out of the collection.
const MAX_PETS: u64 = 2_048;
const MAX_PET_ID: u32 = 1_000_000;
const MAX_LEVEL: u32 = 10;
const MAX_SOULS: u32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PetRecord {
    /// The whole collection: (pet, level) for pets you have, (pet, souls)
    /// toward the next level.
    List {
        levels: Vec<(u32, u32)>,
        souls: Vec<(u32, u32)>,
    },
    Soul {
        pet: u32,
        amount: u32,
    },
    Level {
        pet: u32,
        level: u32,
    },
}

/// Records for one server; `server_id` is 0 when the game never named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetBatch {
    pub server_id: u16,
    pub records: Vec<PetRecord>,
}

#[derive(Default)]
pub(crate) struct PetLog {
    pending: VecDeque<(i64, PetRecord)>,
}

impl PetLog {
    pub fn on_frame(&mut self, frame: &[u8]) {
        if let Some(record) = decode(frame) {
            if self.pending.len() >= MAX_PENDING {
                self.pending.pop_front();
            }
            self.pending.push_back((now_ms(), record));
        }
    }

    /// The records ready to be filed, with the server you play on.
    pub fn take(&mut self, storage: &DataStorage) -> Option<PetBatch> {
        let server_id = storage.fight_server_id();
        let now = now_ms();
        // A fresh login list is held until the server's own record has come.
        let waiting = self
            .pending
            .iter()
            .any(|(at, r)| matches!(r, PetRecord::List { .. }) && now - at < LIST_WAIT_MS);
        if self.pending.is_empty() || waiting {
            return None;
        }
        Some(PetBatch {
            server_id,
            records: self.pending.drain(..).map(|(_, r)| r).collect(),
        })
    }
}

fn decode(frame: &[u8]) -> Option<PetRecord> {
    let len = read_varint(frame, 0);
    if len.length <= 0 {
        return None;
    }
    let at = len.length as usize;
    let opcode: [u8; 2] = frame.get(at..at + 2)?.try_into().ok()?;
    let mut body = Reader {
        bytes: frame,
        at: at + 2,
    };
    match opcode {
        LIST => {
            body.skip(4)?;
            let mut levels = Vec::new();
            for _ in 0..body.count()? {
                let pet = body.pet()?;
                body.skip(4)?;
                levels.push((pet, body.bounded(MAX_LEVEL)?));
            }
            let mut souls = Vec::new();
            for _ in 0..body.count()? {
                let pet = body.pet()?;
                souls.push((pet, body.bounded(MAX_SOULS)?));
            }
            Some(PetRecord::List { levels, souls })
        }
        SOUL => {
            body.skip(1)?;
            let pet = body.pet()?;
            Some(PetRecord::Soul {
                pet,
                amount: body.bounded(MAX_SOULS)?,
            })
        }
        LEVEL => {
            body.skip(2)?;
            let pet = body.pet()?;
            Some(PetRecord::Level {
                pet,
                level: body.bounded(MAX_LEVEL)?,
            })
        }
        _ => None,
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn skip(&mut self, n: usize) -> Option<()> {
        (self.at + n <= self.bytes.len()).then(|| self.at += n)
    }

    fn u32(&mut self) -> Option<u32> {
        let v = u32::from_le_bytes(self.bytes.get(self.at..self.at + 4)?.try_into().ok()?);
        self.at += 4;
        Some(v)
    }

    fn bounded(&mut self, max: u32) -> Option<u32> {
        self.u32().filter(|v| *v <= max)
    }

    fn pet(&mut self) -> Option<u32> {
        self.u32().filter(|id| (1..MAX_PET_ID).contains(id))
    }

    /// A list's length: a varint, so one byte up to 127 pets and two past it.
    fn count(&mut self) -> Option<u64> {
        let v = read_varint(self.bytes, self.at);
        if v.length <= 0 {
            return None;
        }
        self.at += v.length as usize;
        u64::try_from(v.value).ok().filter(|n| *n <= MAX_PETS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame: length byte, opcode, body.
    fn frame(opcode: [u8; 2], body: &[u8]) -> Vec<u8> {
        let mut f = vec![(body.len() + 3) as u8, opcode[0], opcode[1]];
        f.extend_from_slice(body);
        f
    }

    fn le(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn the_login_list_holds_levels_and_souls() {
        let mut body = vec![0xFE, 0x04, 0, 0, 1];
        body.extend(le(&[1097, 1097, 2]));
        body.push(2);
        body.extend(le(&[1034, 28, 1016, 1]));
        body.extend([9, 9, 9]);
        assert_eq!(
            decode(&frame(LIST, &body)),
            Some(PetRecord::List {
                levels: vec![(1097, 2)],
                souls: vec![(1034, 28), (1016, 1)],
            })
        );
    }

    #[test]
    fn a_list_past_127_pets_counts_with_two_bytes() {
        let mut body = vec![0x13, 0x05, 0, 0, 0x87, 0x01];
        for i in 0..135u32 {
            body.extend(le(&[1001 + i, 1001 + i, 1 + i % 3]));
        }
        body.push(0);
        // Too long for a one-byte length: give the frame a two-byte varint.
        let total = body.len() + 4;
        let mut f = vec![(total as u8) | 0x80, (total >> 7) as u8, LIST[0], LIST[1]];
        f.extend(body);
        let Some(PetRecord::List { levels, souls }) = decode(&f) else {
            panic!("a pet list");
        };
        assert_eq!(
            (levels.len(), levels[134], souls.len()),
            (135, (1135, 3), 0)
        );
    }

    #[test]
    fn souls_and_levels_as_they_come() {
        let mut soul = vec![0x01];
        soul.extend(le(&[1016, 1]));
        assert_eq!(
            decode(&frame(SOUL, &soul)),
            Some(PetRecord::Soul {
                pet: 1016,
                amount: 1
            })
        );
        let mut level = vec![0, 0];
        level.extend(le(&[1016, 2]));
        assert_eq!(
            decode(&frame(LEVEL, &level)),
            Some(PetRecord::Level {
                pet: 1016,
                level: 2
            })
        );
        assert_eq!(decode(&frame(SOUL, &[0x01, 0xF8])), None, "truncated");
    }
}
