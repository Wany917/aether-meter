use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("catalog is not valid JSON for this version: {0}")]
    Format(String),
    #[error("catalog entry {0}")]
    Invalid(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RoutineError {
    #[error("no task `{0}`")]
    UnknownTask(String),
    #[error("no reserve `{0}`")]
    UnknownReserve(String),
    #[error("invalid task: {0}")]
    InvalidTask(String),
    #[error("routine file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
