use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PetsError {
    #[error("pet catalog: {0}")]
    Catalog(String),
    #[error("a pet name takes 1 to 40 characters")]
    InvalidName,
    #[error("pets file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
