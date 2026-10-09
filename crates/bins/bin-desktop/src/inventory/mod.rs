//! The Desktop's half of the Inventory: the dialog forms (`form`) that need `gpui`. The `gpui`-free
//! model, rules and stub seed live in `lib_inventory` and are re-exported here, so every
//! `inventory::X` path keeps working.

pub(crate) mod form;

pub use lib_inventory::*;
