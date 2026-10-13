//! Recordings for decoding more of the game: start one, stop it with what
//! the player saw, and find the file to send.

use std::path::PathBuf;

use aether_engine::{NamedValue, RecorderStatus, RecordingNote};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::state::AppState;

fn recordings_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_data_dir()?.join("recordings"))
}

#[tauri::command]
#[specta::specta]
pub fn recording_status(state: State<'_, AppState>) -> RecorderStatus {
    state.engine.recording_status()
}

#[tauri::command]
#[specta::specta]
pub fn start_recording(state: State<'_, AppState>) -> RecorderStatus {
    state.engine.start_recording();
    tracing::info!("recording started");
    state.engine.recording_status()
}

#[tauri::command]
#[specta::specta]
pub fn cancel_recording(state: State<'_, AppState>) -> RecorderStatus {
    state.engine.cancel_recording();
    state.engine.recording_status()
}

/// Stop, save with what the player saw, and show the file in the explorer.
#[tauri::command]
#[specta::specta]
pub fn stop_recording(app: AppHandle, note: RecordingNote) -> Result<String, AppError> {
    let note = RecordingNote {
        scenario: note.scenario.chars().take(32).collect(),
        values: note
            .values
            .into_iter()
            .take(8)
            .map(|v| NamedValue {
                label: v.label.chars().take(40).collect(),
                value: v.value,
            })
            .collect(),
        note: note.note.chars().take(500).collect(),
    };
    let dir = recordings_dir(&app)?;
    let path = app.state::<AppState>().engine.stop_recording(&note, &dir)?;
    if let Err(e) = app.opener().reveal_item_in_dir(&path) {
        tracing::warn!("cannot show {}: {e}", path.display());
    }
    Ok(path.display().to_string())
}

#[tauri::command]
#[specta::specta]
pub fn open_recordings(app: AppHandle) -> Result<(), AppError> {
    let dir = recordings_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| AppError::Engine(e.to_string()))?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::Engine(e.to_string()))
}
