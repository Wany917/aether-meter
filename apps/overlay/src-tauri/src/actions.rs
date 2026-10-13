//! What the tray, the hotkeys and the frontend can all trigger.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::{HubNavigate, HubTab, RoutineChanged, SettingsChanged};
use crate::settings::Settings;
use crate::state::AppState;
use crate::windows::{self, WindowKind};
use crate::{hotkeys, tray};

/// Store new settings and bring every part of the app in line with them.
pub fn save_settings(app: &AppHandle, settings: Settings) -> Result<Settings, AppError> {
    let state = app.state::<AppState>();
    let previous = state.settings.get();
    let saved = state.settings.replace(settings)?;

    if previous.language != saved.language {
        state.engine.set_language(&saved.language);
        let view = state
            .routine
            .lock()
            .view(crate::routine::now_ms(), &saved.language);
        RoutineChanged(view).emit(app)?;
        crate::pets::refresh(app);
    }
    if previous.target_mode != saved.target_mode {
        state.engine.set_target_mode(saved.target_mode);
    }
    windows::apply(app, &saved)?;
    tray::refresh(app, &saved);
    SettingsChanged(saved.clone()).emit(app)?;
    if previous.hotkeys != saved.hotkeys {
        hotkeys::register(app, &saved.hotkeys)?;
    }
    Ok(saved)
}

pub fn set_click_through(app: &AppHandle, enabled: bool) -> Result<Settings, AppError> {
    let mut settings = app.state::<AppState>().settings.get();
    settings.click_through = enabled;
    save_settings(app, settings)
}

pub fn toggle_click_through(app: &AppHandle) -> Result<Settings, AppError> {
    let enabled = app.state::<AppState>().settings.get().click_through;
    set_click_through(app, !enabled)
}

pub fn set_routine_overlay(app: &AppHandle, enabled: bool) -> Result<Settings, AppError> {
    let mut settings = app.state::<AppState>().settings.get();
    settings.routine_overlay = enabled;
    save_settings(app, settings)
}

pub fn set_tracker(app: &AppHandle, enabled: bool) -> Result<Settings, AppError> {
    let mut settings = app.state::<AppState>().settings.get();
    settings.tracker.enabled = enabled;
    save_settings(app, settings)
}

/// Hide both overlays, or bring back the ones the settings want shown.
pub fn toggle_overlays(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let hidden = !state.overlays_hidden.fetch_xor(true, Ordering::Relaxed);
    if hidden {
        for kind in WindowKind::OVERLAYS {
            windows::hide(app, kind)?;
        }
        Ok(())
    } else {
        windows::show(app, WindowKind::Meter)?;
        windows::apply(app, &state.settings.get())
    }
}

pub fn reset(app: &AppHandle) {
    app.state::<AppState>().engine.reset();
}

pub fn open_hub(app: &AppHandle, tab: HubTab, actor_id: Option<i32>) -> Result<(), AppError> {
    windows::show(app, WindowKind::Hub)?;
    HubNavigate { tab, actor_id }.emit(app)?;
    Ok(())
}

pub fn quit(app: &AppHandle) {
    app.state::<AppState>().engine.flush_history();
    app.exit(0);
}
