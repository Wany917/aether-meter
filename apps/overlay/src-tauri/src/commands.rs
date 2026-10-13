//! Commands the frontend can invoke. Thin: each delegates to the engine or to
//! `actions`.

use aether_engine::model::{
    FightSummary, FightView, MeterSnapshot, TimerKind, TimerLabel, TrackerSnapshot,
};
use tauri::{AppHandle, State};

use crate::actions;
use crate::error::AppError;
use crate::events::HubTab;
use crate::settings::Settings;
use crate::state::AppState;
use crate::windows::{self, WindowKind};

#[tauri::command]
#[specta::specta]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
#[specta::specta]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, AppError> {
    actions::save_settings(&app, settings)
}

#[tauri::command]
#[specta::specta]
pub fn set_click_through(app: AppHandle, enabled: bool) -> Result<Settings, AppError> {
    actions::set_click_through(&app, enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_tracker_enabled(app: AppHandle, enabled: bool) -> Result<Settings, AppError> {
    actions::set_tracker(&app, enabled)
}

#[tauri::command]
#[specta::specta]
pub fn set_routine_overlay(app: AppHandle, enabled: bool) -> Result<Settings, AppError> {
    actions::set_routine_overlay(&app, enabled)
}

#[tauri::command]
#[specta::specta]
pub fn get_meter(state: State<'_, AppState>) -> MeterSnapshot {
    state.engine.meter()
}

#[tauri::command]
#[specta::specta]
pub fn get_tracker(state: State<'_, AppState>) -> TrackerSnapshot {
    state.engine.tracker()
}

/// Names and icons for `buff:<id>` / `cooldown:<id>` keys; unknown keys are skipped.
#[tauri::command]
#[specta::specta]
pub fn describe_timers(state: State<'_, AppState>, keys: Vec<String>) -> Vec<TimerLabel> {
    keys.iter()
        .filter_map(|key| {
            let (kind, id) = key.split_once(':')?;
            let kind = match kind {
                "buff" => TimerKind::Buff,
                "cooldown" => TimerKind::Cooldown,
                _ => return None,
            };
            Some(state.engine.timer_label(kind, id.parse().ok()?))
        })
        .collect()
}

#[tauri::command]
#[specta::specta]
pub fn get_live_fight(state: State<'_, AppState>) -> Option<FightView> {
    state.engine.live_fight()
}

#[tauri::command]
#[specta::specta]
pub fn list_fights(state: State<'_, AppState>) -> Vec<FightSummary> {
    state.engine.fights()
}

#[tauri::command]
#[specta::specta]
pub fn get_fight(state: State<'_, AppState>, id: String) -> Result<FightView, AppError> {
    state.engine.saved_fight(&id).map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn delete_fight(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.engine.delete_fight(&id).map_err(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn reset_meter(app: AppHandle) {
    actions::reset(&app);
}

#[tauri::command]
#[specta::specta]
pub fn open_hub(app: AppHandle, tab: HubTab, actor_id: Option<i32>) -> Result<(), AppError> {
    actions::open_hub(&app, tab, actor_id)
}

#[tauri::command]
#[specta::specta]
pub fn hide_window(app: AppHandle, window: WindowKind) -> Result<(), AppError> {
    windows::hide(&app, window)
}

#[tauri::command]
#[specta::specta]
pub fn quit(app: AppHandle) {
    actions::quit(&app);
}
