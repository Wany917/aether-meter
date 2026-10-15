//! The automatic target: the boss fight while it is the one going on, else
//! the target you hit last.
//!
//! The parser's boss mode keeps the last boss of the zone until another one
//! is hit, so a field boss killed half an hour earlier stayed on the meter
//! while you farmed. A boss now gives the meter back once you fight something
//! else well after its last hit.

/// A fight the meter could show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fight {
    pub target: i32,
    /// When anyone last hit the target (the parser's clock).
    pub last_hit_ms: i64,
    pub dead: bool,
}

/// How long after a boss's last hit it keeps the meter while you hit other
/// targets: through adds and phases out of reach while it lives, only the
/// adds left at the kill once it is dead.
const ALIVE_BOSS_HOLD_MS: i64 = 60_000;
const DEAD_BOSS_HOLD_MS: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
    Boss,
    Mine,
}

/// `boss`: the boss fight that is yours or your party's, if any; `mine`: the
/// target you hit last.
pub(crate) fn pick(boss: Option<Fight>, mine: Option<Fight>) -> Pick {
    match (boss, mine) {
        (None, _) => Pick::Mine,
        (Some(_), None) => Pick::Boss,
        (Some(boss), Some(mine)) => {
            let hold = if boss.dead {
                DEAD_BOSS_HOLD_MS
            } else {
                ALIVE_BOSS_HOLD_MS
            };
            if mine.target == boss.target || mine.last_hit_ms <= boss.last_hit_ms + hold {
                Pick::Boss
            } else {
                Pick::Mine
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: i64 = 1000;

    fn fight(target: i32, last_hit_ms: i64, dead: bool) -> Option<Fight> {
        Some(Fight {
            target,
            last_hit_ms,
            dead,
        })
    }

    #[test]
    fn a_boss_fight_holds_through_its_adds() {
        let boss = fight(1, 100 * S, false);
        assert_eq!(pick(boss, fight(2, 100 * S, false)), Pick::Boss);
        // Out of reach for most of a minute, the boss stays.
        assert_eq!(pick(boss, fight(2, 150 * S, false)), Pick::Boss);
        assert_eq!(pick(boss, fight(1, 100 * S, false)), Pick::Boss);
    }

    #[test]
    fn a_dead_boss_stays_until_the_next_fight() {
        let boss = fight(1, 100 * S, true);
        assert_eq!(
            pick(boss, fight(2, 105 * S, false)),
            Pick::Boss,
            "an add left at the kill"
        );
        assert_eq!(
            pick(boss, fight(3, 400 * S, false)),
            Pick::Mine,
            "the next pull"
        );
        assert_eq!(
            pick(boss, None),
            Pick::Boss,
            "nothing hit since: the result stays"
        );
    }

    #[test]
    fn a_boss_left_behind_gives_the_meter_back() {
        let boss = fight(1, 100 * S, false);
        assert_eq!(pick(boss, fight(2, 170 * S, false)), Pick::Mine);
    }

    #[test]
    fn without_a_boss_the_meter_follows_your_target() {
        assert_eq!(pick(None, fight(2, 5 * S, false)), Pick::Mine);
        assert_eq!(pick(None, None), Pick::Mine);
    }
}
