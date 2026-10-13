use serde::Serialize;
use specta::Type;

/// Every error a command can return, as `{ kind, message }` on the frontend.
#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    #[error("{0}")]
    Engine(String),
    #[error("{0}")]
    Settings(String),
    #[error("{0}")]
    Hotkey(String),
    #[error("{0}")]
    Window(String),
    #[error("{0}")]
    Routine(String),
    #[error("{0}")]
    Pets(String),
}

impl From<aether_engine::EngineError> for AppError {
    fn from(e: aether_engine::EngineError) -> Self {
        Self::Engine(e.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::Window(e.to_string())
    }
}
