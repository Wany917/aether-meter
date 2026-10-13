//! Snapshots from the parser: live capture or a replayed capture file.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use a2tools::combat::data_storage::DataStorage;
use a2tools::combat::dps_calculator::DpsCalculator;
use a2tools::combat::ping_tracker::PingTracker;
use parking_lot::Mutex;

use super::Feed;
use crate::dispatch::LinkState;
use crate::gamedata::GameData;
use crate::history::HistoryStore;
use crate::model::{
    EngineStatus, FightSource, FightView, MeterSnapshot, TargetMode, TrackerSnapshot,
};
use crate::snapshot::{self, FightHeader};
use crate::tracker::Tracker;

pub(crate) enum CaptureState {
    Starting,
    Running,
    Failed(String),
}

pub(crate) enum Origin {
    Live,
    Replay {
        file: PathBuf,
        finished: Arc<AtomicBool>,
    },
}

pub(crate) struct LiveFeed {
    pub data: Arc<GameData>,
    pub storage: Arc<DataStorage>,
    pub ping: Arc<PingTracker>,
    pub link: Arc<LinkState>,
    pub tracker: Arc<Mutex<Tracker>>,
    pub history: Arc<HistoryStore>,
    pub origin: Origin,
    /// Set by the thread that opens the capture.
    pub capture: Arc<Mutex<CaptureState>>,
    calculator: Mutex<DpsCalculator>,
    /// The target the meter showed last, for the details view.
    shown_target: AtomicI32,
}

impl LiveFeed {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        data: Arc<GameData>,
        storage: Arc<DataStorage>,
        ping: Arc<PingTracker>,
        link: Arc<LinkState>,
        tracker: Arc<Mutex<Tracker>>,
        history: Arc<HistoryStore>,
        origin: Origin,
        capture: Arc<Mutex<CaptureState>>,
    ) -> Self {
        let calculator = DpsCalculator::new(
            storage.clone(),
            data.skills.clone(),
            data.npcs.clone(),
            ping.clone(),
        );
        Self {
            data,
            storage,
            ping,
            link,
            tracker,
            history,
            origin,
            capture,
            calculator: Mutex::new(calculator),
            shown_target: AtomicI32::new(0),
        }
    }

    fn status(&self) -> EngineStatus {
        match &*self.capture.lock() {
            CaptureState::Failed(reason) => {
                return EngineStatus::CaptureUnavailable {
                    reason: reason.clone(),
                };
            }
            CaptureState::Starting => return EngineStatus::StartingCapture,
            CaptureState::Running => {}
        }
        match &self.origin {
            Origin::Replay { file, finished } => EngineStatus::Replay {
                file: file.file_name().map_or_else(
                    || file.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                ),
                finished: finished.load(Ordering::Relaxed),
            },
            Origin::Live => match self.link.locked_port() {
                Some(server_port) => EngineStatus::Live { server_port },
                None => EngineStatus::WaitingForGame,
            },
        }
    }

    /// A calculator for read-only queries, so details never wait on (or
    /// disturb) the meter's target selection.
    fn reader(&self) -> DpsCalculator {
        DpsCalculator::new(
            self.storage.clone(),
            self.data.skills.clone(),
            self.data.npcs.clone(),
            self.ping.clone(),
        )
    }
}

impl Feed for LiveFeed {
    fn meter(&self) -> MeterSnapshot {
        let dps = self.calculator.lock().get_dps();
        self.shown_target.store(dps.target_id, Ordering::Relaxed);
        let ping = self
            .ping
            .current_ping_ms()
            .and_then(|p| u32::try_from(p).ok());
        snapshot::meter(&dps, self.status(), ping, &self.data)
    }

    fn live_fight(&self) -> Option<FightView> {
        let target_id = self.shown_target.load(Ordering::Relaxed);
        if target_id == 0 {
            return None;
        }
        let reader = self.reader();
        let details = reader.get_target_details(target_id, None);
        let context = reader.get_details_context();
        let target_name = context
            .targets
            .iter()
            .find(|t| t.target_id == target_id)
            .map(|t| t.target_name.clone())
            .unwrap_or_default();
        let header = FightHeader {
            source: FightSource::Live,
            target_name,
            dungeon: self.data.dungeon_label(self.storage.current_dungeon_id()),
            started_at_ms: details.start_time,
            local_actor: self
                .storage
                .local_player_id()
                .and_then(|id| i32::try_from(id).ok()),
        };
        Some(snapshot::fight(
            header,
            &details,
            &context.actors,
            &self.data,
        ))
    }

    fn tracker(&self) -> TrackerSnapshot {
        let own = self
            .storage
            .local_player_id()
            .and_then(|id| i32::try_from(id).ok());
        let mut tracker = self.tracker.lock();
        tracker.set_own_entity(own);
        tracker.snapshot(&self.data)
    }

    fn reset(&self) {
        self.save_fights(true);
        self.calculator.lock().restart_target_selection(true);
        self.shown_target.store(0, Ordering::Relaxed);
        self.tracker.lock().clear();
    }

    fn set_target_mode(&self, mode: TargetMode) {
        self.calculator
            .lock()
            .set_target_selection_mode(mode.parser_id());
    }

    fn save_fights(&self, force: bool) {
        if self.storage.damage_generation() <= 0 {
            return;
        }
        let records = {
            let mut calculator = self.calculator.lock();
            if force {
                calculator.snapshot_boss_fights_force()
            } else {
                calculator.snapshot_boss_fights()
            }
        };
        for record in &records {
            if let Err(e) = self.history.save(record) {
                tracing::warn!("could not save fight {}: {e}", record.id);
            }
        }
    }
}
