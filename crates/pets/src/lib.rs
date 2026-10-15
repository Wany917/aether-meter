//! The pet soul tracker: how far each pet is from its next level, kept up to
//! date from the game's own records (the collection at login, every soul
//! picked up, every level reached), which the engine reads off the capture.
//!
//! A collection is shared by every character on a server, so one is kept per
//! server. What the records do not say (names, genera, souls per level) comes
//! from a community catalog, `catalog/pets.json`; the player can name pets
//! too. Where each pet's souls drop ships with the app, `catalog/pet-sources.json`.

mod catalog;
mod collection;
mod error;
mod pets;
mod sources;
mod store;
mod view;

pub use catalog::{BUNDLED, PetCatalog};
pub use collection::Collection;
pub use error::PetsError;
pub use pets::{PetState, Pets, Record};
pub use sources::{PetSources, Side};
pub use store::Store;
pub use view::{
    ChestView, DropView, GenusView, PetSourcesView, PetView, PetsView, RegionView, TeleportView,
};
