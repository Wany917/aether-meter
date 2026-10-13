//! Saved boss fights: one JSON file per fight plus a small index, so listing
//! never parses the (large) fight files.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use a2tools::entity::fight_record::FightRecord;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;

const INDEX_FILE: &str = "index.json";
const DEFAULT_RETENTION: usize = 300;

/// What the history list needs, language-neutral: names are resolved when
/// listed so a language switch applies to old fights too.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexEntry {
    pub id: String,
    pub boss_name: String,
    pub mob_code: i32,
    pub dungeon_id: i32,
    pub started_at_ms: i64,
    pub duration_ms: i64,
    pub total_damage: i64,
    pub job_ids: Vec<i32>,
    pub is_train: bool,
}

impl IndexEntry {
    fn of(record: &FightRecord) -> Self {
        Self {
            id: record.id.clone(),
            boss_name: record.boss_name.clone(),
            mob_code: record.mob_code,
            dungeon_id: record.dungeon_id,
            started_at_ms: record.start_time_ms,
            duration_ms: record.duration_ms,
            total_damage: i64::from(record.total_damage),
            job_ids: record.job_ids.clone(),
            is_train: record.is_train,
        }
    }
}

pub(crate) struct HistoryStore {
    dir: PathBuf,
    retention: usize,
    /// Serialises index updates; fight files are written whole and renamed.
    index: Mutex<Vec<IndexEntry>>,
}

impl HistoryStore {
    pub fn open(dir: &Path) -> Result<Self, EngineError> {
        fs::create_dir_all(dir).map_err(|source| EngineError::HistoryIo {
            path: dir.to_path_buf(),
            source,
        })?;
        let index_path = dir.join(INDEX_FILE);
        let index = match fs::read_to_string(&index_path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!("history index unreadable ({e}), rebuilding");
                Self::rebuild(dir)
            }),
            Err(_) => Self::rebuild(dir),
        };
        Ok(Self {
            dir: dir.to_path_buf(),
            retention: DEFAULT_RETENTION,
            index: Mutex::new(index),
        })
    }

    /// Newest first.
    pub fn list(&self) -> Vec<IndexEntry> {
        let mut entries = self.index.lock().clone();
        entries.sort_by_key(|e| std::cmp::Reverse(e.started_at_ms));
        entries
    }

    /// Save (or overwrite, as a fight that goes on is saved again) a fight.
    pub fn save(&self, record: &FightRecord) -> Result<(), EngineError> {
        let path = self.fight_path(&record.id)?;
        let json = serde_json::to_vec(record).map_err(|source| EngineError::HistoryFormat {
            path: path.clone(),
            source,
        })?;
        write_atomic(&path, &json)?;

        let mut index = self.index.lock();
        index.retain(|e| e.id != record.id);
        index.push(IndexEntry::of(record));
        index.sort_by_key(|e| std::cmp::Reverse(e.started_at_ms));
        let keep = self.retention.min(index.len());
        for old in index.split_off(keep) {
            let _ = fs::remove_file(self.dir.join(format!("{}.json", old.id)));
        }
        self.write_index(&index)
    }

    pub fn load(&self, id: &str) -> Result<FightRecord, EngineError> {
        let path = self.fight_path(id)?;
        let text = fs::read_to_string(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => EngineError::FightNotFound(id.to_string()),
            _ => EngineError::HistoryIo {
                path: path.clone(),
                source: e,
            },
        })?;
        serde_json::from_str(&text).map_err(|source| EngineError::HistoryFormat { path, source })
    }

    pub fn delete(&self, id: &str) -> Result<(), EngineError> {
        let path = self.fight_path(id)?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => return Err(EngineError::HistoryIo { path, source }),
        }
        let mut index = self.index.lock();
        index.retain(|e| e.id != id);
        self.write_index(&index)
    }

    fn fight_path(&self, id: &str) -> Result<PathBuf, EngineError> {
        let valid = !id.is_empty()
            && id.len() <= 128
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !valid || id == "index" {
            return Err(EngineError::InvalidFightId(id.to_string()));
        }
        Ok(self.dir.join(format!("{id}.json")))
    }

    fn write_index(&self, index: &[IndexEntry]) -> Result<(), EngineError> {
        let path = self.dir.join(INDEX_FILE);
        let json = serde_json::to_vec(index).map_err(|source| EngineError::HistoryFormat {
            path: path.clone(),
            source,
        })?;
        write_atomic(&path, &json)
    }

    /// Recover the index from the fight files themselves.
    fn rebuild(dir: &Path) -> Vec<IndexEntry> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|x| x == "json")
                    && p.file_name().is_some_and(|n| n != INDEX_FILE)
            })
            .filter_map(|p| serde_json::from_str::<FightRecord>(&fs::read_to_string(p).ok()?).ok())
            .map(|r| IndexEntry::of(&r))
            .collect()
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), EngineError> {
    let io = |source| EngineError::HistoryIo {
        path: path.to_path_buf(),
        source,
    };
    let tmp = path.with_extension("json.tmp");
    let mut file = fs::File::create(&tmp).map_err(io)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&tmp, path).map_err(io)
}
