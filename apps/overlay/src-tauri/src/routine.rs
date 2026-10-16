//! The routine and boss tabs' backend: commands, kills seen by the meter,
//! reminders as system notifications, and community catalog updates from the
//! project's repository.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aether_engine::{Kill, OdyleEnergy};
use aether_routine::{Cadence, Catalog, ReminderSettings, RoutineView, TaskGroup};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::RoutineChanged;
use crate::state::AppState;

/// The catalog the community edits; a merged pull request reaches every client.
const COMMUNITY_CATALOG: &str =
    "https://raw.githubusercontent.com/Wany917/aether-meter/main/catalog/routine.json";
/// The reserve the game's Odyle energy fills.
const ODYLE_RESERVE: &str = "odyle-energy";
const CATALOG_CHECK_EVERY: Duration = Duration::from_secs(12 * 3600);
const REMINDER_CHECK_EVERY: Duration = Duration::from_secs(60);

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn language(state: &AppState) -> String {
    state.settings.get().language
}

/// Apply `change`, save, and push the new view to every window.
fn mutate(
    app: &AppHandle,
    change: impl FnOnce(&mut aether_routine::Routine, i64) -> Result<(), aether_routine::RoutineError>,
) -> Result<RoutineView, AppError> {
    let state = app.state::<AppState>();
    let now = now_ms();
    let view = {
        let mut routine = state.routine.lock();
        change(&mut routine, now).map_err(|e| AppError::Routine(e.to_string()))?;
        state
            .routine_store
            .save_state(routine.state())
            .map_err(|e| AppError::Routine(e.to_string()))?;
        routine.view(now, &language(&state))
    };
    RoutineChanged(view.clone()).emit(app)?;
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn get_routine(state: State<'_, AppState>) -> RoutineView {
    state.routine.lock().view(now_ms(), &language(&state))
}

#[tauri::command]
#[specta::specta]
pub fn routine_step(app: AppHandle, id: String, delta: i32) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| r.step_task(&id, delta.clamp(-999, 999), now))
}

#[tauri::command]
#[specta::specta]
pub fn routine_set_reading(
    app: AppHandle,
    id: String,
    amount: u32,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| r.set_reading(&id, amount.min(100_000), now))
}

/// One use spent (negative) or gained, from where the reserve should be now.
#[tauri::command]
#[specta::specta]
pub fn routine_adjust_reading(
    app: AppHandle,
    id: String,
    delta: i32,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| r.adjust_reading(&id, i64::from(delta), now))
}

#[tauri::command]
#[specta::specta]
pub fn routine_add_task(
    app: AppHandle,
    label: String,
    group: TaskGroup,
    cadence: Cadence,
    target: u16,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| {
        r.add_task(&label, group, cadence, target, now).map(|_| ())
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_remove(app: AppHandle, id: String) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| r.remove(&id))
}

#[tauri::command]
#[specta::specta]
pub fn routine_show_hidden(app: AppHandle) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| {
        r.show_hidden();
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_set_membership(app: AppHandle, on: bool) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| {
        r.set_membership(on);
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_set_reminders(
    app: AppHandle,
    reminders: ReminderSettings,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| {
        r.set_reminders(reminders);
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_set_community_updates(app: AppHandle, on: bool) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| {
        r.set_community_updates(on);
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_boss_favorite(
    app: AppHandle,
    id: String,
    on: bool,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| r.set_boss_favorite(&id, on))
}

/// A kill seen elsewhere: now, or at `at_ms`.
#[tauri::command]
#[specta::specta]
pub fn routine_boss_killed(
    app: AppHandle,
    id: String,
    at_ms: Option<i64>,
) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| {
        r.set_boss_kill(&id, Some(at_ms.unwrap_or(now)), now)
    })
}

#[tauri::command]
#[specta::specta]
pub fn routine_boss_forget(app: AppHandle, id: String) -> Result<RoutineView, AppError> {
    mutate(&app, |r, now| r.set_boss_kill(&id, None, now))
}

#[tauri::command]
#[specta::specta]
pub fn routine_set_boss_lead(app: AppHandle, minutes: u8) -> Result<RoutineView, AppError> {
    mutate(&app, |r, _| {
        r.set_boss_lead(minutes);
        Ok(())
    })
}

/// Monsters the meter saw die: start the timers of the bosses among them.
pub fn record_kills(app: &AppHandle, kills: &[Kill]) {
    let state = app.state::<AppState>();
    let view = {
        let mut routine = state.routine.lock();
        let bosses: Vec<String> = kills
            .iter()
            .filter_map(|k| routine.record_kill(k.npc_code, k.at_ms))
            .collect();
        if bosses.is_empty() {
            return;
        }
        tracing::info!("boss kill seen: {}", bosses.join(", "));
        if let Err(e) = state.routine_store.save_state(routine.state()) {
            tracing::warn!("routine state not saved: {e}");
        }
        routine.view(now_ms(), &language(&state))
    };
    let _ = RoutineChanged(view).emit(app);
}

/// The Odyle energy the game sent: the reserve takes it as read, in place
/// of an entered value.
pub fn record_odyle(app: &AppHandle, energy: OdyleEnergy) {
    let state = app.state::<AppState>();
    let now = now_ms();
    let view = {
        let mut routine = state.routine.lock();
        if let Err(e) =
            routine.record_game_reading(ODYLE_RESERVE, energy.basic, energy.charged, now)
        {
            tracing::warn!("Odyle energy not recorded: {e}");
            return;
        }
        if let Err(e) = state.routine_store.save_state(routine.state()) {
            tracing::warn!("routine state not saved: {e}");
        }
        routine.view(now, &language(&state))
    };
    let _ = RoutineChanged(view).emit(app);
}

/// Open a guide or source link from the catalog in the default browser.
#[tauri::command]
#[specta::specta]
pub fn open_link(app: AppHandle, url: String) -> Result<(), AppError> {
    if !url.starts_with("https://") {
        return Err(AppError::Routine(format!("refusing to open `{url}`")));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Routine(e.to_string()))
}

/// Reminders every minute; a catalog check at start, then twice a day.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("routine".into())
        .spawn(move || {
            let mut last_catalog_check: Option<Instant> = None;
            loop {
                if last_catalog_check.is_none_or(|t| t.elapsed() >= CATALOG_CHECK_EVERY) {
                    last_catalog_check = Some(Instant::now());
                    check_community_catalog(&app);
                    crate::pets::check_community_catalog(&app);
                }
                send_due_reminders(&app);
                // Reserves keep recharging: refresh what the tab shows.
                let state = app.state::<AppState>();
                let view = state.routine.lock().view(now_ms(), &language(&state));
                let _ = RoutineChanged(view).emit(&app);
                std::thread::sleep(REMINDER_CHECK_EVERY);
            }
        })
        .expect("spawn routine thread");
}

fn send_due_reminders(app: &AppHandle) {
    let state = app.state::<AppState>();
    let reminders = {
        let mut routine = state.routine.lock();
        let due = routine.due_reminders(now_ms(), &language(&state));
        if !due.is_empty()
            && let Err(e) = state.routine_store.save_state(routine.state())
        {
            tracing::warn!("routine state not saved: {e}");
        }
        due
    };
    for reminder in reminders {
        if let Err(e) = app
            .notification()
            .builder()
            .title(&reminder.title)
            .body(&reminder.body)
            .show()
        {
            tracing::warn!("notification failed: {e}");
        }
    }
}

fn check_community_catalog(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.routine.lock().state().community_updates {
        return;
    }
    let text = match fetch(COMMUNITY_CATALOG) {
        Ok(text) => text,
        Err(e) => {
            tracing::debug!("community catalog unavailable: {e}");
            return;
        }
    };
    let catalog = match Catalog::parse(&text) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("community catalog rejected: {e}");
            return;
        }
    };
    let revision = catalog.revision;
    let view = {
        let mut routine = state.routine.lock();
        if !routine.offer_catalog(catalog) {
            return;
        }
        if let Err(e) = state.routine_store.save_catalog(&text) {
            tracing::warn!("community catalog not cached: {e}");
        }
        routine.view(now_ms(), &language(&state))
    };
    tracing::info!("routine catalog updated to revision {revision}");
    let _ = RoutineChanged(view).emit(app);
}

pub(crate) fn fetch(url: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    agent
        .get(url)
        .header("User-Agent", concat!("Aether/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(512 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())
}
