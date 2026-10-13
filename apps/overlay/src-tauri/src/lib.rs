//! Aether: a game-styled DPS overlay for AION 2.
//!
//! This crate is the desktop shell: windows, tray, hotkeys and the typed
//! bridge to the frontend. Everything about packets and fights lives in
//! `aether-engine`.

mod actions;
mod assets;
mod commands;
mod diagnostics;
mod error;
mod events;
mod hotkeys;
mod routine;
mod settings;
mod state;
mod tick;
mod tray;
mod windows;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use aether_engine::{Engine, EngineConfig, SourceKind};
use aether_routine::{Routine, Store};
use parking_lot::Mutex;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_window_state::StateFlags;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::settings::SettingsStore;
use crate::state::AppState;

/// Commands and events, shared by the app and the bindings export.
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_settings,
            commands::save_settings,
            commands::set_click_through,
            commands::set_tracker_enabled,
            commands::set_routine_overlay,
            commands::get_meter,
            commands::get_tracker,
            commands::describe_timers,
            commands::get_live_fight,
            commands::list_fights,
            commands::get_fight,
            commands::delete_fight,
            commands::reset_meter,
            commands::open_hub,
            commands::hide_window,
            commands::quit,
            diagnostics::report_frontend,
            routine::get_routine,
            routine::routine_step,
            routine::routine_set_reading,
            routine::routine_add_task,
            routine::routine_remove,
            routine::routine_show_hidden,
            routine::routine_set_membership,
            routine::routine_set_reminders,
            routine::routine_set_community_updates,
            routine::open_link,
        ])
        .events(collect_events![
            events::MeterUpdated,
            events::TrackerUpdated,
            events::SettingsChanged,
            events::HubNavigate,
            events::RoutineChanged,
        ])
        // Damage and HP are u64 in Rust but never exceed what a JS number holds
        // exactly: the parser itself sums damage in i32.
        .dangerously_cast_bigints_to_number()
}

pub fn run() {
    diagnostics::init();

    let specta = specta_builder();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = windows::show(app, windows::WindowKind::Meter);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::POSITION | StateFlags::SIZE)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(assets::SCHEME, assets::handle)
        .invoke_handler(specta.invoke_handler())
        .on_page_load(|webview, payload| {
            tracing::info!(
                "window `{}`: page {:?} {}",
                webview.label(),
                payload.event(),
                payload.url()
            );
        })
        .setup(move |app| {
            specta.mount_events(app);
            let handle = app.handle();
            let paths = app.path();

            let settings = SettingsStore::load(paths.app_config_dir()?);
            let current = settings.get();
            let engine = Engine::start(EngineConfig {
                source: source_from_args(),
                data_dir: game_data_dir(paths.resource_dir().ok()),
                history_dir: paths.app_data_dir()?.join("history"),
                language: current.language.clone(),
                target_mode: current.target_mode,
            })?;
            let routine_store = Store::new(&paths.app_data_dir()?);
            let routine = Routine::new(routine_store.load_catalog(), routine_store.load_state());
            app.manage(AppState {
                engine,
                settings,
                overlays_hidden: AtomicBool::new(false),
                routine: Mutex::new(routine),
                routine_store,
            });

            tracing::info!("engine started");
            windows::apply(handle, &current)?;
            tray::create(handle, &current)?;
            tracing::info!("tray ready");
            if let Err(e) = hotkeys::register(handle, &current.hotkeys) {
                tracing::warn!("some shortcuts are unavailable: {e}");
            }
            tick::spawn(handle.clone());
            routine::spawn(handle.clone());
            // Windows that start visible; the others mount when first shown.
            diagnostics::watch_windows_start(&["meter"]);
            tracing::info!("setup done");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Windows hide rather than close: the tray brings them back.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!());
    // Setup errors (missing game data, a window that will not open...) land
    // here: say so instead of vanishing.
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            diagnostics::fatal(&e.to_string());
            return;
        }
    };

    app.run(|app, event| {
        if let RunEvent::ExitRequested { .. } = event
            && let Some(state) = app.try_state::<AppState>()
        {
            state.engine.flush_history();
        }
    });
}

/// `--demo` for a synthetic fight, `--replay <file> [--speed <x>]` for a
/// capture file; live capture otherwise.
fn source_from_args() -> SourceKind {
    let args: Vec<String> = std::env::args().collect();
    let value_of = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
    };
    if args.iter().any(|a| a == "--demo") {
        SourceKind::Demo
    } else if let Some(path) = value_of("--replay") {
        let speed = value_of("--speed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);
        SourceKind::Replay {
            path: PathBuf::from(path),
            speed,
        }
    } else {
        SourceKind::Live
    }
}

/// The bundled data, or the source tree's copy when running from `tauri dev`.
fn game_data_dir(resource_dir: Option<PathBuf>) -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from));
    let candidates = [resource_dir, exe_dir]
        .into_iter()
        .flatten()
        .flat_map(|base| {
            [
                base.join("gamedata"),
                base.join("resources").join("gamedata"),
            ]
        })
        .chain([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("gamedata")]);
    let mut tried = Vec::new();
    for dir in candidates {
        if dir.join("i18n").is_dir() {
            tracing::info!("game data from {}", dir.display());
            return dir;
        }
        tried.push(dir);
    }
    tracing::error!("no game data in any of {tried:?}");
    tried.into_iter().next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    /// `bun run bindings`: regenerate the frontend's typed bindings.
    #[test]
    fn export_bindings() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
        super::specta_builder()
            .export(
                specta_typescript::Typescript::default().header(
                    "/* eslint-disable */\n// Generated by `bun run bindings`. Do not edit.\n",
                ),
                &out,
            )
            .expect("export TypeScript bindings");
    }
}
