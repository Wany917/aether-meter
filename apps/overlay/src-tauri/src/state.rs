use std::sync::atomic::AtomicBool;

use aether_engine::Engine;
use aether_routine::{Routine, Store};
use parking_lot::Mutex;

use crate::settings::SettingsStore;

pub struct AppState {
    pub engine: Engine,
    pub settings: SettingsStore,
    /// Overlays hidden by the toggle hotkey (independent of each window's own setting).
    pub overlays_hidden: AtomicBool,
    pub routine: Mutex<Routine>,
    pub routine_store: Store,
}
