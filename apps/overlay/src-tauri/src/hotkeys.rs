//! Global shortcuts, active while the game has focus.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::actions;
use crate::error::AppError;
use crate::settings::HotkeySettings;

#[derive(Clone, Copy)]
enum Action {
    ClickThrough,
    Reset,
    ToggleOverlays,
}

/// Replace every registered shortcut with the ones in `hotkeys`. An empty
/// accelerator leaves that action without a shortcut.
pub fn register(app: &AppHandle, hotkeys: &HotkeySettings) -> Result<(), AppError> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| AppError::Hotkey(e.to_string()))?;

    let mut failures = Vec::new();
    for (accelerator, action) in [
        (&hotkeys.click_through, Action::ClickThrough),
        (&hotkeys.reset, Action::Reset),
        (&hotkeys.toggle_overlays, Action::ToggleOverlays),
    ] {
        if accelerator.is_empty() {
            continue;
        }
        let registered = shortcuts.on_shortcut(accelerator.as_str(), move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                run(app, action);
            }
        });
        if let Err(e) = registered {
            failures.push(format!("{accelerator}: {e}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Hotkey(failures.join("; ")))
    }
}

fn run(app: &AppHandle, action: Action) {
    let result = match action {
        Action::ClickThrough => actions::toggle_click_through(app).map(|_| ()),
        Action::Reset => {
            actions::reset(app);
            Ok(())
        }
        Action::ToggleOverlays => actions::toggle_overlays(app),
    };
    if let Err(e) = result {
        tracing::warn!("hotkey action failed: {e}");
    }
}
