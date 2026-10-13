//! Aether engine: turns AION 2 network traffic into what the overlay draws.
//!
//! Packet parsing and combat aggregation are the A2Tools DPS Meter parser core
//! (GPL-3.0, <https://github.com/taengu/A2Tools-DPS-Meter>), pinned in the
//! workspace `Cargo.toml`. This crate owns everything around it: packet
//! sources, locking onto the game's connection, the UI model, buff and
//! cooldown tracking, and fight history.

mod capture;
mod clock;
mod dispatch;
mod engine;
mod error;
mod feed;
mod gamedata;
mod history;
mod icons;
mod kills;
pub mod model;
mod pets;
pub mod probe;
mod snapshot;
mod tracker;

pub use engine::{Engine, EngineConfig, SourceKind};
pub use error::EngineError;
pub use gamedata::LANGUAGES;
pub use kills::Kill;
pub use pets::{PetBatch, PetRecord};
