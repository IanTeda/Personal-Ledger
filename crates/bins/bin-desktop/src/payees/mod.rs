//! The Payees module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Payees Settings page and its Dialogs, and `impl PayeesRouter`. Every rule, every piece of text
//! and the keymap are `lib-payees`'; this folder turns gpui keys and clicks into Presenter events
//! and draws the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`PayeesPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are each Dialog Presenter's `DialogView`, hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`payees_page`]).
//!
//! The ports are `lib-database`'s `StoredPayeesPorts`, which `LedgerPorts` hands the page and its
//! Dialogs (#1084).

mod router;
mod view;

pub use router::payees_page;

use lib_payees::presenter::PayeesPagePresenter;

use crate::shell::view::module_page::ModulePage;

/// The Payees Settings page: the page Presenter in the generic page Entity.
pub type PayeesPage = ModulePage<PayeesPagePresenter>;

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

/// What the Payees page's Router is built from for each call (its `PageView::Setup`): its ports,
/// shared with the Ledger and the Bills reads, the Dialog keys every Payees Dialog's hints read,
/// and the Clock its Transactions hand-off dates its filter by.
pub struct PayeesSetup {
    pub ports: lib_payees::SharedPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    pub clock: lib_core::Clock,
}

impl PayeesSetup {
    /// The page's Presenter, which owns the Payees keymap under `[keybindings]`, and its Setup.
    pub fn new(
        ports: lib_payees::SharedPorts,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        clock: lib_core::Clock,
        config: &lib_config::KeyBindingConfig,
    ) -> (PayeesPagePresenter, Self) {
        let presenter = PayeesPagePresenter::new(ports.clone(), config);
        let setup = Self {
            ports,
            dialog_keys,
            clock,
        };
        (presenter, setup)
    }

    /// `storage`'s Payees ports under the default keys, dated by `clock`, for a test that builds
    /// `Root` directly.
    #[cfg(test)]
    pub fn sample(
        storage: &crate::shell::router::LedgerStores,
        clock: &lib_core::Clock,
    ) -> (PayeesPagePresenter, Self) {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.payees.clone(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            clock.clone(),
            &config,
        )
    }
}
