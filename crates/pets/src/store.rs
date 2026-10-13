//! Files in the app's data folder: the collections and the player's names
//! and favorites, and the newest catalog fetched from the community repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::catalog::PetCatalog;
use crate::error::PetsError;
use crate::pets::PetState;

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
        self.dir.join("pets-state.json")
    }

    fn catalog_path(&self) -> PathBuf {
        self.dir.join("pets-catalog.json")
    }

    /// The saved collections; empty ones if the file is missing or unreadable.
    pub fn load_state(&self) -> PetState {
        let path = self.state_path();
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!("{} unreadable, starting over: {e}", path.display());
                PetState::default()
            }),
            Err(_) => PetState::default(),
        }
    }

    pub fn save_state(&self, state: &PetState) -> Result<(), PetsError> {
        let json = serde_json::to_vec_pretty(state).unwrap_or_default();
        self.write(&self.state_path(), &json)
    }

    /// The cached community catalog when it is newer than the bundled one.
    pub fn load_catalog(&self) -> PetCatalog {
        let bundled = PetCatalog::bundled();
        fs::read_to_string(self.catalog_path())
            .ok()
            .and_then(|text| PetCatalog::parse(&text).ok())
            .filter(|cached| cached.revision > bundled.revision)
            .unwrap_or(bundled)
    }

    pub fn save_catalog(&self, json: &str) -> Result<(), PetsError> {
        self.write(&self.catalog_path(), json.as_bytes())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PetsError> {
        let io = |source| PetsError::Io {
            path: path.to_path_buf(),
            source,
        };
        fs::create_dir_all(&self.dir).map_err(io)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes).map_err(io)?;
        fs::rename(&tmp, path).map_err(io)
    }
}
