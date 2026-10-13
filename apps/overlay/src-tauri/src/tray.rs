//! The tray icon: the app's only permanent entry point, as overlays have no
//! taskbar button.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::actions;
use crate::events::HubTab;
use crate::settings::Settings;
use crate::windows::{self, WindowKind};

const TRAY_ID: &str = "aether";

struct Labels {
    meter: &'static str,
    tracker: &'static str,
    details: &'static str,
    history: &'static str,
    routine: &'static str,
    bosses: &'static str,
    pets: &'static str,
    routine_overlay: &'static str,
    settings: &'static str,
    click_through: &'static str,
    reset: &'static str,
    quit: &'static str,
}

fn labels(language: &str) -> Labels {
    match language {
        "fr" => Labels {
            meter: "Afficher le meter",
            tracker: "Buffs et cooldowns",
            details: "Détails du combat",
            history: "Historique",
            routine: "Routine",
            bosses: "Boss",
            pets: "Familiers",
            routine_overlay: "Routine en overlay",
            settings: "Paramètres",
            click_through: "Verrouiller les overlays (clic traversant)",
            reset: "Réinitialiser le combat",
            quit: "Quitter Aether",
        },
        _ => Labels {
            meter: "Show meter",
            tracker: "Buffs & cooldowns",
            details: "Fight details",
            history: "History",
            routine: "Routine",
            bosses: "Bosses",
            pets: "Pets",
            routine_overlay: "Routine overlay",
            settings: "Settings",
            click_through: "Lock overlays (click-through)",
            reset: "Reset fight",
            quit: "Quit Aether",
        },
    }
}

fn menu(app: &AppHandle, settings: &Settings) -> tauri::Result<Menu<Wry>> {
    let l = labels(&settings.language);
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "meter", l.meter, true, None::<&str>)?,
            &CheckMenuItem::with_id(
                app,
                "tracker",
                l.tracker,
                true,
                settings.tracker.enabled,
                None::<&str>,
            )?,
            &CheckMenuItem::with_id(
                app,
                "routine_overlay",
                l.routine_overlay,
                true,
                settings.routine_overlay,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "details", l.details, true, None::<&str>)?,
            &MenuItem::with_id(app, "history", l.history, true, None::<&str>)?,
            &MenuItem::with_id(app, "routine", l.routine, true, None::<&str>)?,
            &MenuItem::with_id(app, "bosses", l.bosses, true, None::<&str>)?,
            &MenuItem::with_id(app, "pets", l.pets, true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", l.settings, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &CheckMenuItem::with_id(
                app,
                "click_through",
                l.click_through,
                true,
                settings.click_through,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "reset", l.reset, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", l.quit, true, None::<&str>)?,
        ],
    )
}

pub fn create(app: &AppHandle, settings: &Settings) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Aether")
        .menu(&menu(app, settings)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = windows::show(tray.app_handle(), WindowKind::Meter);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuild the menu: labels follow the language, checks follow the settings.
pub fn refresh(app: &AppHandle, settings: &Settings) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match menu(app, settings) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => tracing::warn!("tray menu: {e}"),
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    let result = match id {
        "meter" => windows::show(app, WindowKind::Meter),
        "tracker" => {
            let enabled = windows::is_visible(app, WindowKind::Tracker);
            actions::set_tracker(app, !enabled).map(|_| ())
        }
        "details" => actions::open_hub(app, HubTab::Details, None),
        "history" => actions::open_hub(app, HubTab::History, None),
        "routine" => actions::open_hub(app, HubTab::Routine, None),
        "bosses" => actions::open_hub(app, HubTab::Bosses, None),
        "pets" => actions::open_hub(app, HubTab::Pets, None),
        "routine_overlay" => {
            let enabled = windows::is_visible(app, WindowKind::Routine);
            actions::set_routine_overlay(app, !enabled).map(|_| ())
        }
        "settings" => actions::open_hub(app, HubTab::Settings, None),
        "click_through" => actions::toggle_click_through(app).map(|_| ()),
        "reset" => {
            actions::reset(app);
            Ok(())
        }
        "quit" => {
            actions::quit(app);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::warn!("tray action `{id}` failed: {e}");
    }
}
