//! The three windows: two overlays above the game, and the hub.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::error::AppError;
use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WindowKind {
    Meter,
    Tracker,
    Routine,
    Hub,
}

impl WindowKind {
    pub const OVERLAYS: [WindowKind; 3] =
        [WindowKind::Meter, WindowKind::Tracker, WindowKind::Routine];

    pub fn label(self) -> &'static str {
        match self {
            Self::Meter => "meter",
            Self::Tracker => "tracker",
            Self::Routine => "routine",
            Self::Hub => "hub",
        }
    }
}

pub fn get(app: &AppHandle, kind: WindowKind) -> Result<WebviewWindow, AppError> {
    app.get_webview_window(kind.label())
        .ok_or_else(|| AppError::Window(format!("no `{}` window", kind.label())))
}

pub fn show(app: &AppHandle, kind: WindowKind) -> Result<(), AppError> {
    let window = get(app, kind)?;
    window.unminimize()?;
    window.show()?;
    if kind == WindowKind::Hub {
        window.set_focus()?;
    }
    Ok(())
}

pub fn hide(app: &AppHandle, kind: WindowKind) -> Result<(), AppError> {
    get(app, kind)?.hide().map_err(Into::into)
}

pub fn is_visible(app: &AppHandle, kind: WindowKind) -> bool {
    get(app, kind)
        .and_then(|w| w.is_visible().map_err(Into::into))
        .unwrap_or(false)
}

/// Overlays ignore the mouse while click-through is on, so clicks reach the game.
pub fn set_click_through(app: &AppHandle, enabled: bool) -> Result<(), AppError> {
    for kind in WindowKind::OVERLAYS {
        get(app, kind)?.set_ignore_cursor_events(enabled)?;
    }
    Ok(())
}

/// Window state that follows the settings.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), AppError> {
    set_click_through(app, settings.click_through)?;
    for (kind, wanted) in [
        (WindowKind::Tracker, settings.tracker.enabled),
        (WindowKind::Routine, settings.routine_overlay),
    ] {
        if wanted {
            show(app, kind)?;
        } else {
            hide(app, kind)?;
        }
    }
    Ok(())
}
