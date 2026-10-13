use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("game data not found in {0} (expected i18n/skills/*.json)")]
    GameDataMissing(PathBuf),

    #[error("packet capture unavailable: {0}")]
    Capture(String),

    #[error("fight `{0}` not found")]
    FightNotFound(String),

    #[error("invalid fight id `{0}`")]
    InvalidFightId(String),

    #[error("history I/O on {path}: {source}")]
    HistoryIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("recording: {0}")]
    Recording(String),

    #[error("history file {path} is corrupt: {source}")]
    HistoryFormat {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}
