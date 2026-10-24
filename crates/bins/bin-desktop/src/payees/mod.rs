//! The Payees module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Payees Settings page and its Dialogs, and `impl PayeesRouter`. Every rule, every piece of text
//! and the keymap are `lib-payees`'; this folder turns gpui keys and clicks into Presenter events
//! and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`payees_page`]).
//! - `ports.rs` holds the halfway shims: the Categories copied from the Ledger, the forwarding
//!   shim the Payees writers not yet migrated go through, and the Ledger read copy refreshed after
//!   each write (#832).
//!
//! Where the Payees are kept, the SQLite adapter opened at start or memory, is `lib-database`'s
//! `PayeesStore`.

pub mod ports;
mod router;
mod view;

pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::payees_page;
pub use view::page::PayeesPage;

use lib_payees::presenter::PayeesPageEvent;

/// The Payees page as the UI tests read it: names A–Z, and the selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPayeesSnapshot {
    pub payee_names: Vec<String>,
    pub selected_payee: Option<String>,
}

/// What `shell::router::build` hands the Payees module: its ports, shared with the Ledger and the
/// Bills reads, and its keymap under `[keybindings]`.
pub struct PayeesSetup {
    pub ports: DesktopPorts,
    pub keymap: lib_viper::keymap::Keymap<PayeesPageEvent>,
}

impl PayeesSetup {
    pub fn new(ports: DesktopPorts, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            ports,
            keymap: lib_payees::keymap::payees_page_keymap(config),
        }
    }

    /// `storage`'s Payees ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.payees.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
