//! The Units module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Units Settings page and its Dialogs, and `impl UnitsRouter`. Every rule, every piece of text and
//! the keymap are `lib-units`'; this folder turns gpui keys and clicks into Presenter events and
//! draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per Dialog
//!   in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`units_page`]).
//! - `ports.rs` holds the halfway shims: usage and holdings answered from the Ledger, and the
//!   Ledger read copy refreshed after each write (#746).
//!
//! Where the Units are kept, the SQLite adapter opened at start or memory, is `lib-database`'s
//! `UnitsStore`.

pub mod ports;
mod router;
mod view;

use lib_database::UnitsStore;
pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::units_page;
pub(crate) use view::dialog::{Landed, help, refusal};
pub use view::page::UnitsPage;

/// The Units page as the UI tests read it: its view model.
#[doc(hidden)]
pub type SettingsUnitsSnapshot = lib_units::presenter::UnitsPageViewModel;

/// What `shell::router::build` hands the Units module: its ports and its keymap under `[keybindings]`.
pub struct UnitsSetup {
    pub ports: DesktopPorts,
    /// The Unit Deletion module's port over these ports and the Institutions', which Delete unit
    /// writes through (#903).
    pub deletion: lib_unit_deletion::SharedUnitDeletion,
    pub keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
}

impl UnitsSetup {
    pub fn new(
        ports: Box<dyn UnitsStore>,
        accounts: lib_database::SharedAccountsStore,
        inventory: lib_database::SharedInventoryStore,
        institutions: &crate::institutions::ports::DesktopPorts,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        let ports = LedgerBackedPorts::shared(ports, accounts, inventory);
        Self {
            deletion: crate::unit_deletion::deletion(&ports, institutions),
            ports,
            keymap: lib_units::keymap::units_page_keymap(config),
        }
    }

    /// The stub seed in memory under the default keys, reading `storage`'s Accounts and Inventory
    /// and deleting beside its Institutions, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            Box::new(lib_units::fake::InMemoryPorts::stub(chrono::Utc::now())),
            storage.accounts.clone(),
            storage.inventory.clone(),
            &storage.institutions,
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
