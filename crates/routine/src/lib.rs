//! The routine: AION 2's recurring content as checklists that reset with the
//! game, and banked counters (Odyle Energy and the like) projected to their
//! cap so nothing goes to waste.
//!
//! What the content is lives in a community catalog (`catalog/routine.json`),
//! updated by pull request as the game changes; this crate only knows how to
//! count.

mod bosses;
mod catalog;
mod error;
mod routine;
mod schedule;
mod state;
mod store;
mod view;
mod zone;

pub use catalog::{BUNDLED, Catalog, TaskGroup};
pub use error::{CatalogError, RoutineError};
pub use routine::{Reminder, Routine};
pub use schedule::{Cadence, Weekday};
pub use state::{ReminderSettings, UserState};
pub use store::Store;
pub use view::{
    BossRule, BossStatus, BossView, ReserveProjection, ReserveView, RoutineView, TaskView,
};
pub use zone::ServerZone;
