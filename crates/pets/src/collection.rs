//! One server's pets: levels and souls toward the next level, as the game
//! counts them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Collection {
    /// Pet id -> level, for the pets the login list placed above level 0.
    pub levels: BTreeMap<u32, u32>,
    /// Pet id -> souls toward the next level.
    pub souls: BTreeMap<u32, u32>,
    /// When the game last sent the whole collection.
    pub synced_at: Option<i64>,
    /// The pet that last got a soul, and when.
    pub last_gain: Option<(u32, i64)>,
}

impl Collection {
    pub fn level(&self, pet: u32) -> u32 {
        self.levels.get(&pet).copied().unwrap_or(0)
    }

    pub fn souls(&self, pet: u32) -> u32 {
        self.souls.get(&pet).copied().unwrap_or(0)
    }

    /// The login list: the game's own counts replace ours.
    pub fn sync(&mut self, levels: &[(u32, u32)], souls: &[(u32, u32)], now: i64) {
        self.levels = levels.iter().copied().collect();
        self.souls = souls.iter().copied().collect();
        self.synced_at = Some(now);
    }

    /// A soul picked up. The pet levels up once its souls reach what its level
    /// needs, its count starting again from 0 as the game's does: the game's
    /// own level-up record does not come for every level.
    pub fn gain(&mut self, pet: u32, amount: u32, now: i64, needed: impl Fn(u32) -> Option<u32>) {
        let souls = self.souls.entry(pet).or_default();
        *souls = souls.saturating_add(amount);
        self.last_gain = Some((pet, now));
        let level = self.level(pet);
        if needed(level).is_some_and(|n| self.souls(pet) >= n) {
            self.levels.insert(pet, level + 1);
            self.souls.insert(pet, 0);
        }
    }

    /// The game's level-up record; one already counted from souls is skipped.
    pub fn level_up(&mut self, pet: u32, level: u32) {
        if self.level(pet) < level {
            self.levels.insert(pet, level);
            self.souls.insert(pet, 0);
        }
    }

    /// Every pet the game mentioned.
    pub fn pets(&self) -> impl Iterator<Item = u32> + '_ {
        self.levels.keys().chain(self.souls.keys()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEEDED: [u32; 3] = [5, 25, 75];

    fn needed(level: u32) -> Option<u32> {
        NEEDED.get(level as usize).copied()
    }

    #[test]
    fn souls_level_a_pet_up_and_start_again_from_zero() {
        let mut c = Collection::default();
        c.sync(&[(1097, 1)], &[(1097, 24)], 0);
        c.gain(1097, 1, 10, needed);
        assert_eq!((c.level(1097), c.souls(1097)), (2, 0));
        // The game's own record for the same level changes nothing.
        c.gain(1097, 1, 11, needed);
        c.level_up(1097, 2);
        assert_eq!((c.level(1097), c.souls(1097)), (2, 1));
    }

    #[test]
    fn a_new_pet_starts_at_level_zero_and_the_top_level_stops_counting() {
        let mut c = Collection::default();
        for _ in 0..5 {
            c.gain(1016, 1, 0, needed);
        }
        assert_eq!((c.level(1016), c.souls(1016)), (1, 0));
        c.level_up(1016, 3);
        c.gain(1016, 1, 0, needed);
        assert_eq!(
            (c.level(1016), c.souls(1016)),
            (3, 1),
            "no level past the top"
        );
    }
}
