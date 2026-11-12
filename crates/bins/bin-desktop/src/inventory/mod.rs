//! The Inventory module's Desktop half (`docs/development/viper-architecture.md`, #867): the Views
//! for the Inventory Settings page and its Location, Area and Remove Dialogs, and `impl
//! InventoryRouter`. Every rule, every piece of text and the keymap are `lib-inventory`'; this
//! folder turns gpui keys and clicks into Presenter events and draws the view models. The
//! `gpui`-free model, rules and sample seed are re-exported from `lib_inventory` so every
//! `inventory::X` path keeps working.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`InventoryPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are each Dialog Presenter's `DialogView`, hosted in a
//!   generic `ModuleDialog` (ADR-0056).
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

use lib_inventory::presenter::InventoryPagePresenter;

use crate::shell::view::module_page::ModulePage;

/// The Inventory Settings page: the page Presenter in the generic page Entity.
pub type InventoryPage = ModulePage<InventoryPagePresenter>;

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

/// What the Open Ledger hands the Inventory module: the page's Presenter, which owns the Inventory
/// keymap under `[keybindings]`, `lib-database`'s Inventory repository, the Units and Documents its
/// Dialogs quote, and the Dialog keys every Inventory Dialog's hints read.
pub struct InventorySetup {
    pub presenter: InventoryPagePresenter,
    pub inventory: SharedInventory,
    pub reads: lib_database::StoredInventoryReads,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl InventorySetup {
    pub fn new(
        inventory: SharedInventory,
        reads: lib_database::StoredInventoryReads,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: InventoryPagePresenter::new(inventory.clone(), config),
            inventory,
            reads,
            dialog_keys,
        }
    }

    /// `storage`'s Inventory store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.inventory.clone(),
            storage.ports.inventory_reads.clone(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}

/// What the Inventory page's Router is built from for each call (its `PageView::Setup`): the
/// Inventory and the reads and Dialog keys its Dialogs open on.
pub struct InventoryRoutes {
    pub inventory: SharedInventory,
    pub reads: lib_database::StoredInventoryReads,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}
