//! Files in the app's data folder: the player's state, and the newest catalog
//! fetched from the community repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::Catalog;
use crate::error::RoutineError;
use crate::state::UserState;

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    fn state_path(&self) -> PathBuf {
        self.dir.join("routine-state.json")
    }

    fn catalog_path(&self) -> PathBuf {
        self.dir.join("routine-catalog.json")
    }

    /// The player's state; a fresh one if the file is missing or unreadable.
    pub fn load_state(&self) -> UserState {
        let path = self.state_path();
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!("{} unreadable, starting over: {e}", path.display());
                UserState::default()
            }),
            Err(_) => UserState::default(),
        }
    }

    pub fn save_state(&self, state: &UserState) -> Result<(), RoutineError> {
        let json = serde_json::to_vec_pretty(state).unwrap_or_default();
        self.write(&self.state_path(), &json)
    }

    /// The catalog to start from: the cached community one when it is newer
    /// than the bundled one and still valid.
    pub fn load_catalog(&self) -> Catalog {
        let bundled = Catalog::bundled();
        fs::read_to_string(self.catalog_path())
            .ok()
            .and_then(|text| Catalog::parse(&text).ok())
            .filter(|cached| cached.revision > bundled.revision)
            .unwrap_or(bundled)
    }

    pub fn save_catalog(&self, json: &str) -> Result<(), RoutineError> {
        self.write(&self.catalog_path(), json.as_bytes())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), RoutineError> {
        let io = |source| RoutineError::Io {
            path: path.to_path_buf(),
            source,
        };
        fs::create_dir_all(&self.dir).map_err(io)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes).map_err(io)?;
        fs::rename(&tmp, path).map_err(io)
    }
}
