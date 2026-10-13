//! Game time: when periods reset and when banked counters tick.
//!
//! Global servers reset at 07:00 UTC (16:00 in Korea), every day for daily
//! content and on Wednesday for weekly content; the hour comes from the
//! catalog so it can follow the game. Everything here is epoch milliseconds
//! in UTC, so daylight saving never shifts a reset.
//!
//! Recharges are assumed to tick on a grid anchored on the daily reset
//! (07:00, 10:00, 13:00... for a 3 h recharge). The game does not document the
//! anchor; entering a fresh value after playing re-syncs the projection anyway.

use serde::{Deserialize, Serialize};

pub const HOUR_MS: i64 = 3_600_000;
pub const DAY_MS: i64 = 24 * HOUR_MS;
pub const WEEK_MS: i64 = 7 * DAY_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    fn index(self) -> i64 {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }
}

/// When the content behind a task comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Cadence {
    Daily,
    Weekly { day: Weekday },
}

/// The server's daily reset hour, and every date computed from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetClock {
    offset_ms: i64,
}

impl ResetClock {
    pub fn new(reset_hour_utc: u8) -> Self {
        Self {
            offset_ms: i64::from(reset_hour_utc % 24) * HOUR_MS,
        }
    }

    pub fn daily_period_start(self, now: i64) -> i64 {
        (now - self.offset_ms).div_euclid(DAY_MS) * DAY_MS + self.offset_ms
    }

    pub fn weekly_period_start(self, now: i64, day: Weekday) -> i64 {
        let today = self.daily_period_start(now);
        // 1970-01-01 was a Thursday (index 3).
        let weekday = ((today - self.offset_ms) / DAY_MS + 3).rem_euclid(7);
        today - (weekday - day.index()).rem_euclid(7) * DAY_MS
    }

    /// Start of the period of `cadence` containing `now`.
    pub fn period_start(self, cadence: Cadence, now: i64) -> i64 {
        match cadence {
            Cadence::Daily => self.daily_period_start(now),
            Cadence::Weekly { day } => self.weekly_period_start(now, day),
        }
    }

    pub fn next_reset(self, cadence: Cadence, now: i64) -> i64 {
        match cadence {
            Cadence::Daily => self.daily_period_start(now) + DAY_MS,
            Cadence::Weekly { day } => self.weekly_period_start(now, day) + WEEK_MS,
        }
    }

    /// Recharge instants in `(from, to]` on a grid of `interval_ms` anchored on the reset.
    pub fn ticks_between(self, from: i64, to: i64, interval_ms: i64) -> i64 {
        if to <= from {
            return 0;
        }
        self.grid_index(to, interval_ms) - self.grid_index(from, interval_ms)
    }

    /// The `n`-th recharge instant after `from` (n >= 1).
    pub fn nth_tick_after(self, from: i64, n: i64, interval_ms: i64) -> i64 {
        self.offset_ms + (self.grid_index(from, interval_ms) + n) * interval_ms
    }

    fn grid_index(self, t: i64, interval_ms: i64) -> i64 {
        (t - self.offset_ms).div_euclid(interval_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-14 (a Wednesday) at `h`:`m` UTC.
    fn wed(h: i64, m: i64) -> i64 {
        1_791_936_000_000 + h * HOUR_MS + m * 60_000
    }

    #[test]
    fn the_test_date_is_a_wednesday() {
        assert_eq!(((wed(0, 0) / DAY_MS) + 3).rem_euclid(7), 2);
    }

    const CLOCK: ResetClock = ResetClock {
        offset_ms: 7 * HOUR_MS,
    };

    #[test]
    fn days_turn_over_at_seven_utc() {
        assert_eq!(CLOCK.daily_period_start(wed(6, 59)), wed(7, 0) - DAY_MS);
        assert_eq!(CLOCK.daily_period_start(wed(7, 0)), wed(7, 0));
        assert_eq!(
            CLOCK.next_reset(Cadence::Daily, wed(12, 0)),
            wed(7, 0) + DAY_MS
        );
        assert_eq!(ResetClock::new(7), CLOCK);
    }

    #[test]
    fn weeks_turn_over_on_wednesday_morning() {
        let wednesday = Cadence::Weekly {
            day: Weekday::Wednesday,
        };
        assert_eq!(
            CLOCK.period_start(wednesday, wed(6, 59)),
            wed(7, 0) - WEEK_MS
        );
        assert_eq!(CLOCK.period_start(wednesday, wed(7, 0)), wed(7, 0));
        assert_eq!(
            CLOCK.period_start(wednesday, wed(7, 0) + 3 * DAY_MS),
            wed(7, 0)
        );
        let sunday = Cadence::Weekly {
            day: Weekday::Sunday,
        };
        assert_eq!(
            CLOCK.period_start(sunday, wed(12, 0)),
            wed(7, 0) - 3 * DAY_MS
        );
    }

    #[test]
    fn ticks_land_on_a_grid_anchored_on_the_reset() {
        let three_h = 3 * HOUR_MS;
        assert_eq!(CLOCK.ticks_between(wed(9, 0), wed(10, 0), three_h), 1);
        assert_eq!(CLOCK.ticks_between(wed(10, 0), wed(12, 59), three_h), 0);
        assert_eq!(
            CLOCK.ticks_between(wed(7, 30), wed(7, 30) + DAY_MS, three_h),
            8
        );
        assert_eq!(CLOCK.nth_tick_after(wed(8, 0), 1, three_h), wed(10, 0));
        assert_eq!(CLOCK.nth_tick_after(wed(10, 0), 2, three_h), wed(16, 0));
    }
}
