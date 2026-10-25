//! The Institutions module's Desktop half (`docs/development/viper-architecture.md`): the Views
//! for the Institutions Settings page and its Add Dialog, and `impl InstitutionsRouter`. Every
//! rule, every piece of text and the keymap are `lib-institutions`'; this folder turns gpui keys
//! and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds the Add Dialog's Presenter in an Entity in the Dialog host.
//! - `router.rs` is the Router and the module's entry point ([`institutions_page`]).
//! - `ports.rs` holds the halfway shims: the Units copied from the Ledger, and the Ledger read
//!   copy refreshed after each write (#850).
//!
//! Where the Institutions are kept, the SQLite adapter opened at start or memory, is
//! `lib-database`'s `InstitutionsStore`.

pub mod ports;
mod router;
mod view;

pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::institutions_page;
pub use view::page::InstitutionsPage;

use lib_institutions::presenter::InstitutionsPageEvent;

/// The Institutions page: `(name, account type)` rows in table order, and the selected row's name
/// (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInstitutionsSnapshot {
    pub institutions: Vec<(String, String)>,
    pub selected_institution: Option<String>,
}

/// What `shell::router::build` hands the Institutions module: its ports, shared with the Ledger
/// store, and its keymap under `[keybindings]`.
pub struct InstitutionsSetup {
    pub ports: DesktopPorts,
    pub keymap: lib_viper::keymap::Keymap<InstitutionsPageEvent>,
}

impl InstitutionsSetup {
    pub fn new(ports: DesktopPorts, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            ports,
            keymap: lib_institutions::keymap::institutions_page_keymap(config),
        }
    }

    /// `storage`'s Institutions ports under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.institutions.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
