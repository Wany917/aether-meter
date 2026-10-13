//! Server wall clocks, for schedules the game announces in local time.
//!
//! Europe servers run on Berlin time: UTC+1, and UTC+2 from the last Sunday
//! of March to the last Sunday of October (both at 01:00 UTC, the EU rule).
//! Coded here rather than pulled from a time zone database: one fixed rule.

use serde::{Deserialize, Serialize};

use crate::schedule::{DAY_MS, HOUR_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ServerZone {
    Utc,
    /// Central European time, summer time included.
    Europe,
}

impl ServerZone {
    /// Offset from UTC at instant `t`.
    pub fn offset_at(self, t: i64) -> i64 {
        match self {
            Self::Utc => 0,
            Self::Europe if eu_summer_time(t) => 2 * HOUR_MS,
            Self::Europe => HOUR_MS,
        }
    }

    /// The instant a wall-clock time (ms since 1970-01-01 00:00 local) happens.
    pub fn to_utc(self, local: i64) -> i64 {
        let guess = local - self.offset_at(local);
        local - self.offset_at(guess)
    }

    /// The local day `t` falls in, as days since 1970-01-01.
    pub fn local_day(self, t: i64) -> i64 {
        (t + self.offset_at(t)).div_euclid(DAY_MS)
    }
}

fn eu_summer_time(t: i64) -> bool {
    let (year, _, _) = civil_from_days(t.div_euclid(DAY_MS));
    let start = last_sunday(year, 3) * DAY_MS + HOUR_MS;
    let end = last_sunday(year, 10) * DAY_MS + HOUR_MS;
    (start..end).contains(&t)
}

/// Days since 1970-01-01 of the last Sunday of `month`.
fn last_sunday(year: i64, month: u32) -> i64 {
    let last = days_from_civil(year, month, 31);
    // 1970-01-01 was a Thursday: Sunday is 3 days on.
    last - (last - 3).rem_euclid(7)
}

/// Howard Hinnant's `days_from_civil` (proleptic Gregorian calendar).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn utc(days: i64, hour: i64) -> i64 {
        days * DAY_MS + hour * HOUR_MS
    }

    #[test]
    fn calendar_round_trips() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 10, 14), 20_740);
        assert_eq!(civil_from_days(20_740), (2026, 10, 14));
        assert_eq!(civil_from_days(days_from_civil(2028, 2, 29)), (2028, 2, 29));
    }

    #[test]
    fn berlin_switches_on_the_last_sundays_of_march_and_october() {
        let europe = ServerZone::Europe;
        // 2026: summer time from March 29 to October 25.
        let (march, october) = (days_from_civil(2026, 3, 29), days_from_civil(2026, 10, 25));
        assert_eq!(
            (last_sunday(2026, 3), last_sunday(2026, 10)),
            (march, october)
        );
        assert_eq!(europe.offset_at(utc(march, 0)), HOUR_MS);
        assert_eq!(europe.offset_at(utc(march, 1)), 2 * HOUR_MS);
        assert_eq!(europe.offset_at(utc(october, 0)), 2 * HOUR_MS);
        assert_eq!(europe.offset_at(utc(october, 1)), HOUR_MS);
    }

    #[test]
    fn wall_clock_times_map_to_the_right_instant() {
        let europe = ServerZone::Europe;
        let summer = days_from_civil(2026, 10, 16); // a Friday
        let winter = days_from_civil(2026, 10, 30);
        // 21:00 in Berlin: 19:00 UTC in summer, 20:00 UTC in winter.
        assert_eq!(europe.to_utc(utc(summer, 21)), utc(summer, 19));
        assert_eq!(europe.to_utc(utc(winter, 21)), utc(winter, 20));
        // 00:30 UTC on a summer night is already the next day in Berlin.
        assert_eq!(europe.local_day(utc(summer, 0) + HOUR_MS / 2), summer);
        assert_eq!(europe.local_day(utc(summer, 23)), summer + 1);
        assert_eq!(ServerZone::Utc.to_utc(utc(summer, 21)), utc(summer, 21));
    }
}
