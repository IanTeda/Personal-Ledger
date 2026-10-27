//! The Ledger's Inventory: Locations, their Areas and the Items in them (`GLOSSARY.md`'s Location,
//! Area and Inventory Item). A VIPER module (`docs/development/viper-architecture.md`, #847, #865):
//! the Entity with its rules and stub seed (`entity`), one Interactor per intent (`interactor`)
//! and the repository port (`protocols`). `fake` holds the in-memory port; `contract` the checks
//! both it and a stored adapter keep. No UI toolkit and no I/O.
//!
//! Presenters (`presenter`), one per View and Dialog of the Settings page, take events and build
//! plain-data view models with their text from `msg`; `keymap` holds the page's default keys. They
//! ask for navigation through `InventoryRouter`. No UI toolkit and no I/O; the Desktop draws the
//! view models (`bin-desktop/src/inventory/`).
//!
//! Documents depends on this module (a Document Link names an Item by id), never the reverse, so
//! removing a Location reports the Items that went and leaves the Documents to drop their Links.
//! That makes Remove Location a `lib-ledger` write, not an Interactor here; Remove Area only moves
//! Items within the Inventory, so it is one.

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use entity::*;
pub use protocols::{
    InventoryChange, InventoryDialog, InventoryOutcome, InventoryReassign, InventoryRepository,
    InventoryRouter, InventoryRow, InventoryUnits, LocationRemoved, LocationUsage, LocationUse,
    SharedInventory, UnitChoice,
};
