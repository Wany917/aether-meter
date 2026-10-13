use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use a2tools::capture::captured_payload::CapturedPayload;
use a2tools::capture::frame_tap;
use a2tools::combat::data_storage::DataStorage;
use a2tools::combat::ping_tracker::PingTracker;
use parking_lot::Mutex;

use crate::capture;
use crate::dispatch::{Dispatcher, LinkState};
use crate::error::EngineError;
use crate::feed::{CaptureState, DemoFeed, Feed, LiveFeed, Origin};
use crate::gamedata::GameData;
use crate::history::HistoryStore;
use crate::kills::{Kill, KillLog};
use crate::model::{
    FightSummary, FightView, MeterSnapshot, TargetMode, TimerKind, TimerLabel, TrackerSnapshot,
};
use crate::snapshot;
use crate::tracker::{self, Tracker};

const CHANNEL_CAPACITY: usize = 4096;
const AUTOSAVE_EVERY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub enum SourceKind {
    /// Every network device of this machine (Npcap on Windows).
    Live,
    /// A `.pcap`/`.pcapng` capture, replayed at `speed` times real time.
    Replay { path: PathBuf, speed: f64 },
    /// A synthetic fight, no capture at all.
    Demo,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub source: SourceKind,
    /// The game data folder (`i18n/`, `abnormals.json`, ...).
    pub data_dir: PathBuf,
    pub history_dir: PathBuf,
    pub language: String,
    pub target_mode: TargetMode,
}

/// The running engine. Dropping it stops capture.
pub struct Engine {
    data: Arc<GameData>,
    history: Arc<HistoryStore>,
    feed: Arc<dyn Feed>,
    running: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Engine {
    pub fn start(config: EngineConfig) -> Result<Self, EngineError> {
        let data = Arc::new(GameData::load(&config.data_dir, &config.language)?);
        let history = Arc::new(HistoryStore::open(&config.history_dir)?);
        let running = Arc::new(AtomicBool::new(true));

        let (feed, mut threads): (Arc<dyn Feed>, Vec<JoinHandle<()>>) = match config.source {
            SourceKind::Demo => (
                Arc::new(DemoFeed::new(data.clone())) as Arc<dyn Feed>,
                Vec::new(),
            ),
            SourceKind::Live => start_parser(&data, &history, &running, None)?,
            SourceKind::Replay { path, speed } => {
                start_parser(&data, &history, &running, Some((path, speed)))?
            }
        };
        feed.set_target_mode(config.target_mode);
        threads.push(spawn_autosave(Arc::downgrade(&feed), running.clone()));

        Ok(Self {
            data,
            history,
            feed,
            running,
            threads,
        })
    }

    pub fn meter(&self) -> MeterSnapshot {
        self.feed.meter()
    }

    pub fn live_fight(&self) -> Option<FightView> {
        self.feed.live_fight()
    }

    pub fn tracker(&self) -> TrackerSnapshot {
        self.feed.tracker()
    }

    /// Monsters killed within sight since the last call (live capture only).
    pub fn take_kills(&self) -> Vec<Kill> {
        self.feed.take_kills()
    }

    pub fn timer_label(&self, kind: TimerKind, id: u32) -> TimerLabel {
        tracker::label(kind, id, &self.data)
    }

    /// Saves the fights on the meter, then starts it over.
    pub fn reset(&self) {
        self.feed.reset();
    }

    pub fn set_target_mode(&self, mode: TargetMode) {
        self.feed.set_target_mode(mode);
    }

    pub fn language(&self) -> String {
        self.data.language()
    }

    pub fn set_language(&self, language: &str) {
        self.data.set_language(language);
    }

    pub fn fights(&self) -> Vec<FightSummary> {
        self.history
            .list()
            .iter()
            .map(|e| snapshot::summary(e, &self.data))
            .collect()
    }

    pub fn saved_fight(&self, id: &str) -> Result<FightView, EngineError> {
        let record = self.history.load(id)?;
        Ok(snapshot::saved_fight(&record, &self.data))
    }

    pub fn delete_fight(&self, id: &str) -> Result<(), EngineError> {
        self.history.delete(id)
    }

    /// Save every fight on the meter now (call before quitting).
    pub fn flush_history(&self) {
        self.feed.save_fights(true);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        frame_tap::set(None);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

type Started = (Arc<dyn Feed>, Vec<JoinHandle<()>>);

fn start_parser(
    data: &Arc<GameData>,
    history: &Arc<HistoryStore>,
    running: &Arc<AtomicBool>,
    replay: Option<(PathBuf, f64)>,
) -> Result<Started, EngineError> {
    let storage = Arc::new(DataStorage::new());
    storage.set_abnormal_stack_limits(data.stack_limits.clone());
    let ping = Arc::new(PingTracker::new());
    let link = Arc::new(LinkState::default());

    let tracker = Arc::new(Mutex::new(Tracker::default()));
    let kills = Arc::new(Mutex::new(KillLog::default()));
    let (tap_tracker, tap_kills) = (tracker.clone(), kills.clone());
    frame_tap::set(Some(Arc::new(move |frame: &[u8]| {
        tap_tracker.lock().on_frame(frame);
        tap_kills.lock().on_frame(frame);
    })));

    let (tx, rx) = sync_channel::<CapturedPayload>(CHANNEL_CAPACITY);
    let finished = Arc::new(AtomicBool::new(false));
    let origin = match &replay {
        None => Origin::Live,
        Some((path, _)) => Origin::Replay {
            file: path.clone(),
            finished: finished.clone(),
        },
    };

    // Opening the capture library and every adapter can take seconds, or wait
    // on Npcap's administrator prompt: never on the caller's (UI) thread.
    let capture_state = Arc::new(Mutex::new(CaptureState::Starting));
    let mut threads = Vec::new();
    {
        let (state, running) = (capture_state.clone(), running.clone());
        threads.push(
            std::thread::Builder::new()
                .name("capture-start".into())
                .spawn(move || {
                    let capture = match replay {
                        None => capture::start_live(tx, running),
                        Some((path, speed)) => {
                            capture::start_replay(&path, speed, tx, running, finished)
                        }
                    };
                    match capture {
                        Ok(capture) => {
                            *state.lock() = CaptureState::Running;
                            // Keep the device threads, so dropping the engine waits for them.
                            for thread in capture.threads {
                                let _ = thread.join();
                            }
                        }
                        Err(e) => {
                            tracing::error!("capture unavailable: {e}");
                            *state.lock() = CaptureState::Failed(e);
                        }
                    }
                })
                .map_err(|e| EngineError::Capture(e.to_string()))?,
        );
    }

    let dispatcher = Dispatcher::new(storage.clone(), data.clone(), ping.clone(), link.clone());
    let dispatch_running = running.clone();
    threads.push(
        std::thread::Builder::new()
            .name("dispatch".into())
            .spawn(move || dispatcher.run(rx, dispatch_running))
            .map_err(|e| EngineError::Capture(e.to_string()))?,
    );

    let feed = Arc::new(LiveFeed::new(
        data.clone(),
        storage.clone(),
        ping,
        link,
        tracker,
        kills,
        history.clone(),
        origin,
        capture_state,
    ));
    // Zone changes and party ends clear combat data: save first. Weak, as the
    // storage would otherwise keep the feed that owns it alive.
    let weak = Arc::downgrade(&feed);
    storage.set_before_reset(move || {
        if let Some(feed) = weak.upgrade() {
            feed.save_fights(true);
        }
    });
    let feed: Arc<dyn Feed> = feed;
    Ok((feed, threads))
}

fn spawn_autosave(feed: Weak<dyn Feed>, running: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("autosave".into())
        .spawn(move || {
            let tick = Duration::from_millis(500);
            let mut waited = Duration::ZERO;
            while running.load(Ordering::Relaxed) {
                std::thread::sleep(tick);
                waited += tick;
                if waited < AUTOSAVE_EVERY {
                    continue;
                }
                waited = Duration::ZERO;
                match feed.upgrade() {
                    Some(feed) => feed.save_fights(false),
                    None => break,
                }
            }
        })
        .expect("spawn autosave thread")
}
