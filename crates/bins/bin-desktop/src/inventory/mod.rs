//! The Inventory module's Desktop half (`docs/development/viper-architecture.md`, #867): the Views
//! for the Inventory Settings page and its Location, Area and Remove Dialogs, and `impl
//! InventoryRouter`. Every rule, every piece of text and the keymap are `lib-inventory`'; this
//! folder turns gpui keys and clicks into Presenter events and draws the view models. The
//! `gpui`-free model, rules and sample seed are re-exported from `lib_inventory` so every
//! `inventory::X` path keeps working.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`inventory_page`]).
//!
//! The Inventory repository is `lib-database`'s `StoredInventoryPorts`; the Units and Documents its
//! Dialogs quote are read through `StoredInventoryReads` (#867, #1001, #1083).
//!
//! The Inventory destination (`Noun::Inventory`) is a placeholder that shares nothing with the
//! page. Where the Inventory is kept, the SQLite adapter opened at start or memory, is
//! `lib-database`'s `InventoryStore`.

mod router;
mod view;

pub use lib_inventory::*;
pub use router::inventory_page;
pub use view::page::InventoryPage;

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

/// What the Open Ledger hands the Inventory module: `lib-database`'s Inventory repository, the Units and Documents its Dialogs quote, and its keymap under `[keybindings]`.
pub struct InventorySetup {
    pub inventory: SharedInventory,
    pub reads: lib_database::StoredInventoryReads,
    pub keymap: lib_viper::keymap::Keymap<InventoryPageEvent>,
}

impl InventorySetup {
    pub fn new(
        inventory: SharedInventory,
        reads: lib_database::StoredInventoryReads,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            inventory,
            reads,
            keymap: keymap::inventory_page_keymap(config),
        }
    }

    /// `storage`'s Inventory store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.inventory_ports.clone(),
            storage.inventory_reads(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
