//! The pets tab's backend: the collection records the engine reads, the
//! player's names and favorites, and community catalog updates.

use aether_engine::{PetBatch, PetRecord};
use aether_pets::{PetCatalog, PetsView, Record};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::PetsChanged;
use crate::routine::{fetch, now_ms};
use crate::state::AppState;

/// The catalog the community edits; a merged pull request reaches every client.
const COMMUNITY_CATALOG: &str =
    "https://raw.githubusercontent.com/Wany917/aether-meter/main/catalog/pets.json";

fn view(state: &AppState) -> PetsView {
    let pets = state.pets.lock();
    let server_name = state.engine.server_name(pets.current_server());
    pets.view(&state.settings.get().language, server_name)
}

/// Apply `change`, save, and push the new view.
fn mutate(
    app: &AppHandle,
    change: impl FnOnce(&mut aether_pets::Pets) -> Result<(), aether_pets::PetsError>,
) -> Result<PetsView, AppError> {
    let state = app.state::<AppState>();
    {
        let mut pets = state.pets.lock();
        change(&mut pets).map_err(|e| AppError::Pets(e.to_string()))?;
        state
            .pets_store
            .save_state(pets.state())
            .map_err(|e| AppError::Pets(e.to_string()))?;
    }
    let view = view(&state);
    PetsChanged(view.clone()).emit(app)?;
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn get_pets(state: State<'_, AppState>) -> PetsView {
    view(&state)
}

#[tauri::command]
#[specta::specta]
pub fn pets_favorite(app: AppHandle, id: u32, on: bool) -> Result<PetsView, AppError> {
    mutate(&app, |pets| {
        pets.set_favorite(id, on);
        Ok(())
    })
}

/// The player's own name for a pet; an empty one restores the catalog's.
#[tauri::command]
#[specta::specta]
pub fn pets_rename(app: AppHandle, id: u32, name: String) -> Result<PetsView, AppError> {
    mutate(&app, |pets| pets.rename(id, &name))
}

/// Push the view again (names follow the language).
pub fn refresh(app: &AppHandle) {
    let _ = PetsChanged(view(&app.state::<AppState>())).emit(app);
}

/// Records the engine read: file them under their server and save.
pub fn record(app: &AppHandle, batch: PetBatch) {
    let records: Vec<Record> = batch
        .records
        .into_iter()
        .map(|r| match r {
            PetRecord::List { levels, souls } => Record::List { levels, souls },
            PetRecord::Soul { pet, amount } => Record::Soul { pet, amount },
            PetRecord::Level { pet, level } => Record::Level { pet, level },
        })
        .collect();
    let state = app.state::<AppState>();
    {
        let mut pets = state.pets.lock();
        if !pets.apply(batch.server_id, records, now_ms()) {
            return;
        }
        if let Err(e) = state.pets_store.save_state(pets.state()) {
            tracing::warn!("pets not saved: {e}");
        }
    }
    let _ = PetsChanged(view(&state)).emit(app);
}

/// Fetch a newer pet catalog, when community updates are on.
pub fn check_community_catalog(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.routine.lock().state().community_updates {
        return;
    }
    let text = match fetch(COMMUNITY_CATALOG) {
        Ok(text) => text,
        Err(e) => {
            tracing::debug!("community pet catalog unavailable: {e}");
            return;
        }
    };
    let catalog = match PetCatalog::parse(&text) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("community pet catalog rejected: {e}");
            return;
        }
    };
    let revision = catalog.revision;
    {
        let mut pets = state.pets.lock();
        if !pets.offer_catalog(catalog) {
            return;
        }
        if let Err(e) = state.pets_store.save_catalog(&text) {
            tracing::warn!("community pet catalog not cached: {e}");
        }
    }
    tracing::info!("pet catalog updated to revision {revision}");
    let _ = PetsChanged(view(&state)).emit(app);
}
