//! The Payees module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Payees Settings page and its Dialogs, and `impl PayeesRouter`. Every rule, every piece of text
//! and the keymap are `lib-payees`'; this folder turns gpui keys and clicks into Presenter events
//! and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`payees_page`]).
//!
//! The ports are `lib-database`'s `StoredPayeesPorts`, which `LedgerPorts` hands the page and its
//! Dialogs (#1084).

mod router;
mod view;

pub use router::payees_page;
pub use view::page::PayeesPage;

use lib_payees::presenter::PayeesPageEvent;

/// The Payees page as the UI tests read it: names A–Z, each row's match-rule count as drawn, and
/// the selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPayeesSnapshot {
    pub payee_names: Vec<String>,
    /// `1 rule`, parallel to `payee_names`.
    pub payee_rules: Vec<String>,
    pub selected_payee: Option<String>,
}

/// What `shell::router::build` hands the Payees module: its ports, shared with the Ledger and the
/// Bills reads, and its keymap under `[keybindings]`.
pub struct PayeesSetup {
    pub ports: lib_payees::SharedPorts,
    pub keymap: lib_viper::keymap::Keymap<PayeesPageEvent>,
}

impl PayeesSetup {
    pub fn new(ports: lib_payees::SharedPorts, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            ports,
            keymap: lib_payees::keymap::payees_page_keymap(config),
        }
    }

    /// `storage`'s Payees ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.payee_ports.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
