mod demo;
mod live;

pub(crate) use demo::DemoFeed;
pub(crate) use live::{CaptureState, LiveFeed, Origin};

use crate::kills::Kill;
use crate::model::{FightView, MeterSnapshot, TargetMode, TrackerSnapshot};

/// Where snapshots come from: the parser, or a synthetic fight for UI work.
pub(crate) trait Feed: Send + Sync {
    fn meter(&self) -> MeterSnapshot;
    /// Every player's breakdown on the target the meter shows.
    fn live_fight(&self) -> Option<FightView>;
    fn tracker(&self) -> TrackerSnapshot;
    /// Save what can be saved, then start the meter over.
    fn reset(&self);
    fn set_target_mode(&self, mode: TargetMode);
    /// Save ended boss fights; `force` saves the ones still going too.
    fn save_fights(&self, force: bool);
    /// Monsters killed in sight of a live capture since the last call.
    fn take_kills(&self) -> Vec<Kill>;
}
