use crate::catalog::{Catalog, ReserveDef, TaskGroup, localize};
use crate::error::RoutineError;
use crate::schedule::{Cadence, HOUR_MS, ResetClock, Weekday};
use crate::state::{CustomTask, Progress, Reading, ReminderSettings, UserState};
use crate::view::{ReserveProjection, ReserveView, RoutineView, TaskView};

const WEEKLY: Cadence = Cadence::Weekly {
    day: Weekday::Wednesday,
};
const MAX_NOTIFIED: usize = 200;
const MAX_CUSTOM_TASKS: usize = 50;

/// A reminder ready to be shown as a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub title: String,
    pub body: String,
}

/// The catalog and what the player did with it.
pub struct Routine {
    pub(crate) catalog: Catalog,
    pub(crate) state: UserState,
}

impl Routine {
    pub fn new(catalog: Catalog, state: UserState) -> Self {
        Self { catalog, state }
    }

    pub fn state(&self) -> &UserState {
        &self.state
    }

    pub fn catalog_revision(&self) -> u32 {
        self.catalog.revision
    }

    /// Take a newer catalog revision; an older or equal one is ignored.
    pub fn offer_catalog(&mut self, catalog: Catalog) -> bool {
        if catalog.revision <= self.catalog.revision {
            return false;
        }
        self.catalog = catalog;
        true
    }

    fn clock(&self) -> ResetClock {
        ResetClock::new(self.catalog.reset_hour_utc)
    }

    pub fn view(&self, now: i64, language: &str) -> RoutineView {
        let clock = self.clock();
        let membership = self.state.membership;
        let visible = |id: &str| !self.state.hidden.iter().any(|h| h == id);

        let catalog_tasks = self
            .catalog
            .tasks
            .iter()
            .filter(|t| visible(&t.id) && (membership || !t.membership_only))
            .map(|t| TaskView {
                id: t.id.clone(),
                label: localize(&t.label, language),
                note: t.note.as_ref().map(|n| localize(n, language)),
                group: t.group,
                cadence: t.cadence,
                target: t.target,
                progress: self.progress(&t.id, t.cadence, t.target, now),
                next_reset_ms: clock.next_reset(t.cadence, now),
                guide: t.guide.clone(),
                custom: false,
                verified: t.verified,
            });
        let custom_tasks = self.state.custom_tasks.iter().map(|t| TaskView {
            id: t.id.clone(),
            label: t.label.clone(),
            note: None,
            group: t.group,
            cadence: t.cadence,
            target: t.target,
            progress: self.progress(&t.id, t.cadence, t.target, now),
            next_reset_ms: clock.next_reset(t.cadence, now),
            guide: None,
            custom: true,
            verified: true,
        });

        let reserves = self
            .catalog
            .reserves
            .iter()
            .filter(|r| visible(&r.id))
            .map(|r| {
                let cap = r.cap_for(membership);
                ReserveView {
                    id: r.id.clone(),
                    label: localize(&r.label, language),
                    note: r.note.as_ref().map(|n| localize(n, language)),
                    guide: r.guide.clone(),
                    verified: r.verified,
                    cap,
                    regen: r.regen,
                    every_hours: r.every_hours,
                    step: r.step.unwrap_or(1),
                    reading: self
                        .state
                        .readings
                        .get(&r.id)
                        .map(|reading| project(clock, r, cap, *reading, now)),
                }
            })
            .collect();

        let catalog_ids = self
            .catalog
            .tasks
            .iter()
            .map(|t| &t.id)
            .chain(self.catalog.reserves.iter().map(|r| &r.id));
        RoutineView {
            now_ms: now,
            next_daily_reset_ms: clock.next_reset(Cadence::Daily, now),
            next_weekly_reset_ms: clock.next_reset(WEEKLY, now),
            membership,
            reminders: self.state.reminders,
            community_updates: self.state.community_updates,
            catalog_revision: self.catalog.revision,
            catalog_updated: self.catalog.updated.clone(),
            tasks: catalog_tasks.chain(custom_tasks).collect(),
            reserves,
            bosses: self.boss_views(now, language),
            boss_lead_minutes: self.state.bosses.lead_minutes,
            hidden_count: catalog_ids.filter(|id| !visible(id)).count() as u32,
        }
    }

    /// Done count this period: progress from an earlier period does not count.
    fn progress(&self, id: &str, cadence: Cadence, target: u16, now: i64) -> u16 {
        self.state
            .progress
            .get(id)
            .filter(|p| p.period_start == self.clock().period_start(cadence, now))
            .map_or(0, |p| p.count.min(target))
    }

    fn task_shape(&self, id: &str) -> Option<(Cadence, u16)> {
        self.catalog
            .task(id)
            .map(|t| (t.cadence, t.target))
            .or_else(|| {
                self.state
                    .custom_tasks
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| (t.cadence, t.target))
            })
    }

    /// Add `delta` (usually +1 or -1) to a task's count for this period.
    pub fn step_task(&mut self, id: &str, delta: i32, now: i64) -> Result<(), RoutineError> {
        let (cadence, target) = self
            .task_shape(id)
            .ok_or_else(|| RoutineError::UnknownTask(id.to_string()))?;
        let current = i32::from(self.progress(id, cadence, target, now));
        let count = (current + delta).clamp(0, i32::from(target)) as u16;
        let period_start = self.clock().period_start(cadence, now);
        self.state.progress.insert(
            id.to_string(),
            Progress {
                count,
                period_start,
            },
        );
        Ok(())
    }

    /// The value the player reads in game right now.
    pub fn set_reading(&mut self, id: &str, amount: u32, now: i64) -> Result<(), RoutineError> {
        if self.catalog.reserve(id).is_none() {
            return Err(RoutineError::UnknownReserve(id.to_string()));
        }
        self.state
            .readings
            .insert(id.to_string(), Reading { amount, at: now });
        Ok(())
    }

    /// Add `delta` (negative to spend) to where a reserve should be now.
    pub fn adjust_reading(&mut self, id: &str, delta: i64, now: i64) -> Result<(), RoutineError> {
        let def = self
            .catalog
            .reserve(id)
            .ok_or_else(|| RoutineError::UnknownReserve(id.to_string()))?;
        let reading = *self
            .state
            .readings
            .get(id)
            .ok_or_else(|| RoutineError::NotEntered(id.to_string()))?;
        let cap = def.cap_for(self.state.membership);
        let current = i64::from(project(self.clock(), def, cap, reading, now).amount);
        let amount = (current + delta).clamp(0, i64::from(u32::MAX)) as u32;
        self.state
            .readings
            .insert(id.to_string(), Reading { amount, at: now });
        Ok(())
    }

    pub fn add_task(
        &mut self,
        label: &str,
        group: TaskGroup,
        cadence: Cadence,
        target: u16,
        now: i64,
    ) -> Result<String, RoutineError> {
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 60 {
            return Err(RoutineError::InvalidTask(
                "the name needs 1 to 60 characters".into(),
            ));
        }
        if !(1..=999).contains(&target) {
            return Err(RoutineError::InvalidTask(
                "the count must be 1 to 999".into(),
            ));
        }
        if self.state.custom_tasks.len() >= MAX_CUSTOM_TASKS {
            return Err(RoutineError::InvalidTask(format!(
                "at most {MAX_CUSTOM_TASKS} tasks of your own"
            )));
        }
        let id = format!("custom-{now}");
        self.state.custom_tasks.push(CustomTask {
            id: id.clone(),
            label: label.to_string(),
            group,
            cadence,
            target,
        });
        Ok(id)
    }

    /// Delete one of the player's tasks, or hide a catalog entry.
    pub fn remove(&mut self, id: &str) -> Result<(), RoutineError> {
        if let Some(index) = self.state.custom_tasks.iter().position(|t| t.id == id) {
            self.state.custom_tasks.remove(index);
            self.state.progress.remove(id);
            return Ok(());
        }
        if self.catalog.task(id).is_none() && self.catalog.reserve(id).is_none() {
            return Err(RoutineError::UnknownTask(id.to_string()));
        }
        if !self.state.hidden.iter().any(|h| h == id) {
            self.state.hidden.push(id.to_string());
        }
        Ok(())
    }

    pub fn show_hidden(&mut self) {
        self.state.hidden.clear();
    }

    pub fn set_membership(&mut self, on: bool) {
        self.state.membership = on;
    }

    pub fn set_reminders(&mut self, reminders: ReminderSettings) {
        self.state.reminders = ReminderSettings {
            enabled: reminders.enabled,
            before_reset_hours: reminders.before_reset_hours.clamp(1, 24),
            before_cap_hours: reminders.before_cap_hours.clamp(1, 48),
        };
    }

    pub fn set_community_updates(&mut self, on: bool) {
        self.state.community_updates = on;
    }

    /// Reminders that became due, each returned once.
    pub fn due_reminders(&mut self, now: i64, language: &str) -> Vec<Reminder> {
        if !self.state.reminders.enabled {
            return Vec::new();
        }
        let fr = language == "fr";
        let view = self.view(now, language);
        let reset_lead = i64::from(self.state.reminders.before_reset_hours) * HOUR_MS;
        let cap_lead = i64::from(self.state.reminders.before_cap_hours) * HOUR_MS;
        let mut due: Vec<(String, Reminder)> = Vec::new();

        // Unfinished tasks before the reset that would wipe them, one reminder per reset.
        let mut by_reset: Vec<(i64, Vec<&TaskView>)> = Vec::new();
        for task in view
            .tasks
            .iter()
            .filter(|t| t.progress < t.target && t.next_reset_ms - now <= reset_lead)
        {
            match by_reset
                .iter_mut()
                .find(|(reset, _)| *reset == task.next_reset_ms)
            {
                Some((_, tasks)) => tasks.push(task),
                None => by_reset.push((task.next_reset_ms, vec![task])),
            }
        }
        for (reset, tasks) in by_reset {
            let hours = ((reset - now) as f64 / HOUR_MS as f64).ceil().max(1.0) as i64;
            let names = tasks
                .iter()
                .take(3)
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let more = tasks.len().saturating_sub(3);
            let title = if fr {
                format!("Reset dans {hours} h : {} à finir", tasks.len())
            } else {
                format!("Reset in {hours} h: {} left to do", tasks.len())
            };
            let body = match (more, fr) {
                (0, _) => names,
                (n, true) => format!("{names} et {n} autre(s)"),
                (n, false) => format!("{names} and {n} more"),
            };
            due.push((format!("reset:{reset}"), Reminder { title, body }));
        }

        for reserve in &view.reserves {
            let Some(reading) = &reserve.reading else {
                continue;
            };
            if let Some(full_at) = reading.full_at_ms.filter(|at| at - now <= cap_lead) {
                let hours = ((full_at - now) as f64 / HOUR_MS as f64).ceil().max(1.0) as i64;
                let title = if fr {
                    format!("{} plein dans {hours} h", reserve.label)
                } else {
                    format!("{} full in {hours} h", reserve.label)
                };
                let body = if fr {
                    format!(
                        "{} sur {}. Utilise-le avant de perdre la recharge.",
                        reading.amount, reserve.cap
                    )
                } else {
                    format!(
                        "{} of {}. Spend it before the recharge goes to waste.",
                        reading.amount, reserve.cap
                    )
                };
                due.push((
                    format!("cap:{}:{full_at}", reserve.id),
                    Reminder { title, body },
                ));
            }
            if let Some(full_since) = reading.full_since_ms {
                let title = if fr {
                    format!("{} est plein", reserve.label)
                } else {
                    format!("{} is full", reserve.label)
                };
                let body = if fr {
                    format!("La recharge est perdue tant qu'il reste à {}.", reserve.cap)
                } else {
                    format!("Recharge is lost while it stays at {}.", reserve.cap)
                };
                due.push((
                    format!("full:{}:{full_since}", reserve.id),
                    Reminder { title, body },
                ));
            }
        }

        due.extend(self.boss_reminders(&view.bosses, now, fr));

        let mut fresh = Vec::new();
        for (key, reminder) in due {
            if !self.state.notified.contains(&key) {
                self.state.notified.push(key);
                fresh.push(reminder);
            }
        }
        let excess = self.state.notified.len().saturating_sub(MAX_NOTIFIED);
        self.state.notified.drain(..excess);
        fresh
    }
}

/// Where a banked counter should be now, from the last value entered.
fn project(
    clock: ResetClock,
    def: &ReserveDef,
    cap: u32,
    reading: Reading,
    now: i64,
) -> ReserveProjection {
    let interval = i64::from(def.every_hours) * HOUR_MS;
    let regen = i64::from(def.regen);
    let ticks = clock.ticks_between(reading.at, now, interval);
    let (amount, cap) = (i64::from(reading.amount), i64::from(cap));

    let (amount, full_at, full_since, wasted) = if amount >= cap {
        // Already full (refill items can even push it past the cap).
        (amount, None, Some(reading.at), regen * ticks)
    } else {
        let needed = (cap - amount + regen - 1) / regen;
        if ticks >= needed {
            let overflow = amount + needed * regen - cap;
            (
                cap,
                None,
                Some(clock.nth_tick_after(reading.at, needed, interval)),
                overflow + regen * (ticks - needed),
            )
        } else {
            (
                amount + ticks * regen,
                Some(clock.nth_tick_after(reading.at, needed, interval)),
                None,
                0,
            )
        }
    };
    ReserveProjection {
        amount: amount.clamp(0, i64::from(u32::MAX)) as u32,
        recorded_at_ms: reading.at,
        full_at_ms: full_at,
        full_since_ms: full_since,
        wasted: wasted.clamp(0, i64::from(u32::MAX)) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::DAY_MS;

    /// 2026-10-14, a Wednesday, at `h`:00 UTC.
    fn wed(h: i64) -> i64 {
        1_791_936_000_000 + h * HOUR_MS
    }

    fn routine() -> Routine {
        Routine::new(Catalog::bundled(), UserState::default())
    }

    #[test]
    fn task_progress_clears_at_the_reset() {
        let mut r = routine();
        let id = r
            .add_task("Farm", TaskGroup::Daily, Cadence::Daily, 5, wed(8))
            .unwrap();
        r.step_task(&id, 1, wed(8)).unwrap();
        r.step_task(&id, 1, wed(9)).unwrap();
        let done = |r: &Routine, now| {
            r.view(now, "fr")
                .tasks
                .iter()
                .find(|t| t.id == id)
                .unwrap()
                .progress
        };
        assert_eq!(done(&r, wed(20)), 2);
        assert_eq!(done(&r, wed(7) + DAY_MS), 0, "next day");
        r.step_task(&id, 9, wed(10)).unwrap();
        assert_eq!(done(&r, wed(10)), 5, "capped at the target");
    }

    #[test]
    fn odyle_projects_to_its_cap_and_counts_what_is_lost() {
        let mut r = routine();
        r.set_reading("odyle-energy", 530, wed(8)).unwrap();
        let odyle = |r: &Routine, now| {
            r.view(now, "en")
                .reserves
                .into_iter()
                .find(|v| v.id == "odyle-energy")
                .unwrap()
                .reading
                .unwrap()
        };
        let soon = odyle(&r, wed(9));
        assert_eq!(
            (soon.amount, soon.full_at_ms),
            (530, Some(wed(13))),
            "15 at 10:00, 15 at 13:00"
        );
        let later = odyle(&r, wed(19));
        assert_eq!(
            (later.amount, later.full_since_ms, later.wasted),
            (560, Some(wed(13)), 30),
            "16:00 and 19:00 lost"
        );

        r.set_membership(true);
        assert_eq!(odyle(&r, wed(19)).amount, 590, "the member cap is higher");
    }

    #[test]
    fn quick_adjustments_start_from_the_projected_value() {
        let mut r = routine();
        assert!(
            r.adjust_reading("odyle-energy", -40, wed(8)).is_err(),
            "nothing entered yet"
        );
        r.set_reading("odyle-energy", 200, wed(8)).unwrap();
        // 10:00 and 13:00 added 30: a cube at 14:00 starts from 230.
        r.adjust_reading("odyle-energy", -40, wed(14)).unwrap();
        let odyle = r
            .view(wed(14), "en")
            .reserves
            .into_iter()
            .find(|v| v.id == "odyle-energy")
            .unwrap();
        assert_eq!((odyle.step, odyle.reading.unwrap().amount), (40, 190));
        r.adjust_reading("odyle-energy", -9_999, wed(14)).unwrap();
        let empty = r
            .view(wed(14), "en")
            .reserves
            .into_iter()
            .find(|v| v.id == "odyle-energy")
            .unwrap();
        assert_eq!(empty.reading.unwrap().amount, 0, "never below zero");
    }

    #[test]
    fn reminders_fire_once_before_the_reset_and_before_the_cap() {
        let mut r = routine();
        let before_reset = wed(7) + DAY_MS - HOUR_MS;
        let first = r.due_reminders(before_reset, "fr");
        assert!(
            first.iter().any(|m| m.title.starts_with("Reset dans 1 h")),
            "{first:?}"
        );
        assert!(
            r.due_reminders(before_reset + 60_000, "fr").is_empty(),
            "sent once"
        );

        r.set_reading("odyle-energy", 550, wed(8)).unwrap();
        let cap = r.due_reminders(wed(9), "en");
        assert!(
            cap.iter().any(|m| m.title == "Odyle Energy full in 1 h"),
            "{cap:?}"
        );
    }

    #[test]
    fn catalog_entries_hide_and_custom_tasks_delete() {
        let mut r = routine();
        let id = r
            .add_task("Farm kinah", TaskGroup::Daily, Cadence::Daily, 1, wed(8))
            .unwrap();
        r.remove("secret-mission").unwrap();
        r.remove(&id).unwrap();
        let view = r.view(wed(9), "en");
        assert!(
            view.tasks
                .iter()
                .all(|t| t.id != "secret-mission" && t.id != id)
        );
        assert_eq!(view.hidden_count, 1);
        r.show_hidden();
        assert!(
            r.view(wed(9), "en")
                .tasks
                .iter()
                .any(|t| t.id == "secret-mission")
        );
        assert!(
            r.add_task("  ", TaskGroup::Daily, Cadence::Daily, 1, wed(8))
                .is_err()
        );
    }

    #[test]
    fn only_newer_catalogs_replace_the_current_one() {
        let mut r = routine();
        let mut newer = Catalog::bundled();
        newer.revision += 1;
        assert!(!r.offer_catalog(Catalog::bundled()));
        assert!(r.offer_catalog(newer));
    }
}
