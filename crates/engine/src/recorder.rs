//! Recordings for decoding more of the game, made from the app.
//!
//! While a recording runs, every frame of the game's stream (decompressed,
//! as the parser reads it) is kept with its time. The player does one thing
//! whose effect they know, such as opening a hidden cube with 12 keys and
//! keeping 11, then stops: the frames go to a JSON Lines file, the values
//! they saw on its first line. `aether-probe` reads it back (`--find`).
//!
//! ```text
//! {"format":"aether-recording","version":1,"app":"0.1.9","startedMs":...,"scenario":"cube","values":[...],"note":"..."}
//! {"t":1520,"f":"0b4236ccc8010003"}
//! ```
//!
//! A recording holds what the game sent: names of players nearby, chat.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clock::now_ms;
use crate::error::EngineError;

/// A recording stops keeping frames past this, so a forgotten one stays small.
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    pub active: bool,
    pub started_ms: Option<i64>,
    pub frames: u32,
    pub bytes: u32,
    /// The size limit was reached: frames since then were not kept.
    pub full: bool,
}

/// What the player did and saw, saved at the top of the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RecordingNote {
    /// What was done: `cube`, `odyle`, `duty`, `other`.
    pub scenario: String,
    pub values: Vec<NamedValue>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NamedValue {
    pub label: String,
    pub value: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Header<'a> {
    format: &'static str,
    version: u32,
    app: &'static str,
    started_ms: i64,
    #[serde(flatten)]
    note: &'a RecordingNote,
}

#[derive(Default)]
pub(crate) struct Recorder {
    active: Option<Active>,
}

struct Active {
    started_ms: i64,
    frames: Vec<(i64, Vec<u8>)>,
    bytes: usize,
    full: bool,
}

impl Recorder {
    pub fn on_frame(&mut self, frame: &[u8]) {
        let Some(active) = &mut self.active else {
            return;
        };
        if active.bytes + frame.len() > MAX_BYTES {
            active.full = true;
            return;
        }
        active.bytes += frame.len();
        active
            .frames
            .push((now_ms() - active.started_ms, frame.to_vec()));
    }

    /// Start over: a recording already running is dropped.
    pub fn start(&mut self) {
        self.active = Some(Active {
            started_ms: now_ms(),
            frames: Vec::new(),
            bytes: 0,
            full: false,
        });
    }

    pub fn cancel(&mut self) {
        self.active = None;
    }

    pub fn status(&self) -> RecorderStatus {
        match &self.active {
            None => RecorderStatus {
                active: false,
                started_ms: None,
                frames: 0,
                bytes: 0,
                full: false,
            },
            Some(a) => RecorderStatus {
                active: true,
                started_ms: Some(a.started_ms),
                frames: u32::try_from(a.frames.len()).unwrap_or(u32::MAX),
                bytes: u32::try_from(a.bytes).unwrap_or(u32::MAX),
                full: a.full,
            },
        }
    }

    /// Stop and write the file into `dir`; returns its path.
    pub fn stop(&mut self, note: &RecordingNote, dir: &Path) -> Result<PathBuf, EngineError> {
        let active = self
            .active
            .take()
            .ok_or_else(|| EngineError::Recording("no recording is running".into()))?;
        let io = |e: std::io::Error| EngineError::Recording(e.to_string());
        fs::create_dir_all(dir).map_err(io)?;
        let path = dir.join(format!(
            "aether-{}-{}.jsonl",
            file_word(&note.scenario),
            active.started_ms / 1000
        ));
        let mut out = BufWriter::new(fs::File::create(&path).map_err(io)?);
        let header = Header {
            format: "aether-recording",
            version: 1,
            app: env!("CARGO_PKG_VERSION"),
            started_ms: active.started_ms,
            note,
        };
        serde_json::to_writer(&mut out, &header)
            .map_err(|e| EngineError::Recording(e.to_string()))?;
        writeln!(out).map_err(io)?;
        for (t, frame) in &active.frames {
            writeln!(out, "{{\"t\":{t},\"f\":\"{}\"}}", hex(frame)).map_err(io)?;
        }
        out.flush().map_err(io)?;
        tracing::info!(
            "recording saved: {} ({} frames)",
            path.display(),
            active.frames.len()
        );
        Ok(path)
    }
}

/// A scenario as a file name word: `cube`, `other`.
fn file_word(text: &str) -> String {
    let word: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase();
    if word.is_empty() {
        "recording".into()
    } else {
        word
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(text, "{b:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_is_written_and_read_back() {
        let mut recorder = Recorder::default();
        recorder.on_frame(&[1, 2, 3]);
        assert_eq!(recorder.status().frames, 0, "nothing kept before start");
        recorder.start();
        recorder.on_frame(&[0x0b, 0x42, 0x36, 0xcc]);
        recorder.on_frame(&[0x05, 0x47, 0x38]);
        assert_eq!((recorder.status().frames, recorder.status().bytes), (2, 7));

        let dir = std::env::temp_dir().join(format!("aether-recorder-{}", std::process::id()));
        let note = RecordingNote {
            scenario: "Cube!".into(),
            values: vec![NamedValue {
                label: "keys before".into(),
                value: 12,
            }],
            note: String::new(),
        };
        let path = recorder.stop(&note, &dir).unwrap();
        assert!(!recorder.status().active);
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("aether-cube-")
        );

        let frames = crate::probe::read_recording(&path).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            (frames[0].opcode, frames[1].opcode),
            ([0x42, 0x36], [0x47, 0x38])
        );
        assert_eq!(crate::probe::recording_values(&path).unwrap(), vec![12]);
        let _ = fs::remove_dir_all(dir);
    }
}
