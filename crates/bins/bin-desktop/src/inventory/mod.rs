//! The Inventory module's Desktop half (`docs/development/viper-architecture.md`, #867): the Views
//! for the Inventory Settings page and its Location, Area and Remove Dialogs, and `impl
//! InventoryRouter`. Every rule, every piece of text and the keymap are `lib-inventory`'; this
//! folder turns gpui keys and clicks into Presenter events and draws the view models. The
//! `gpui`-free model, rules and stub seed are re-exported from `lib_inventory` so every
//! `inventory::X` path keeps working.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`inventory_page`]).
//! - `ports.rs` holds the store as the Presenters share it, the forwarding shim Remove Location
//!   still goes through, and the ports answered from the Ledger: the Units, the Documents and the
//!   reassign port (#867).
//!
//! The Inventory destination (`Noun::Inventory`) is a placeholder that shares nothing with the
//! page. Where the Inventory is kept, the SQLite adapter opened at start or memory, is
//! `lib-database`'s `InventoryStore`.

pub mod ports;
mod router;
mod view;

pub use lib_inventory::*;
pub use router::inventory_page;
pub use view::page::InventoryPage;

use lib_database::SharedInventoryStore;
use lib_inventory::presenter::InventoryPageEvent;

/// The Inventory page as the UI tests read it (ADR-0030): the visible rows and the selected one,
/// as `location:<name>` or `area:<name>`, each Location's Areas in order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInventorySnapshot {
    pub inventory_rows: Vec<String>,
    pub selected_inventory: Option<String>,
    pub inventory_areas: Vec<(String, Vec<String>)>,
    /// The open row menu: the row it is on, as above, and its entries' slugs in order.
    pub inventory_menu: Option<(String, Vec<String>)>,
    pub inventory_locations: Vec<(String, String, Option<String>)>,
    /// Each Location's id, in the same order.
    pub inventory_location_ids: Vec<lib_core::LocationId>,
    /// Each Area's name with its Item count, in page order.
    pub inventory_area_items: Vec<(String, usize)>,
}

/// What `shell::router::build` hands the Inventory module: the store it shares with the Ledger's
/// forwarding shim and read copy, and its keymap under `[keybindings]`.
pub struct InventorySetup {
    pub store: SharedInventoryStore,
    pub keymap: lib_viper::keymap::Keymap<InventoryPageEvent>,
}

impl InventorySetup {
    pub fn new(store: SharedInventoryStore, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            store,
            keymap: keymap::inventory_page_keymap(config),
        }
    }

    /// `storage`'s Inventory store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.inventory.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
