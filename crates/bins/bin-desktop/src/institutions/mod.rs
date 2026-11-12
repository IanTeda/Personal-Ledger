//! The Institutions module's Desktop half (`docs/development/viper-architecture.md`): the Views
//! for the Institutions Settings page and its Add Dialog, and `impl InstitutionsRouter`. Every
//! rule, every piece of text and the keymap are `lib-institutions`'; this folder turns gpui keys
//! and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`InstitutionsPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are each Dialog Presenter's `DialogView`, hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`institutions_page`]).
//!
//! The ports are `lib-database`'s `StoredInstitutionsPorts`, which `LedgerPorts` hands the page and
//! its Dialogs (#1084).

mod router;
mod view;

pub use router::institutions_page;

use lib_institutions::presenter::InstitutionsPagePresenter;

use crate::shell::view::module_page::ModulePage;

/// The Institutions Settings page: the page Presenter in the generic page Entity.
pub type InstitutionsPage = ModulePage<InstitutionsPagePresenter>;

/// The Institutions page: `(name, products)` rows in table order, and the selected row's name
/// (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInstitutionsSnapshot {
    pub institutions: Vec<(String, String)>,
    pub selected_institution: Option<String>,
    /// The slugs of the open row menu's entries, `None` while no menu is open.
    pub menu: Option<Vec<&'static str>>,
}

/// What the Open Ledger hands the Institutions module: the page's Presenter, which owns the
/// Institutions keymap under `[keybindings]`, its ports, shared with the Ledger store, and the
/// Dialog keys every Institutions Dialog's hints read.
pub struct InstitutionsSetup {
    pub presenter: InstitutionsPagePresenter,
    pub ports: lib_institutions::SharedPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl InstitutionsSetup {
    pub fn new(
        ports: lib_institutions::SharedPorts,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: InstitutionsPagePresenter::new(ports.clone(), config),
            ports,
            dialog_keys,
        }
    }

    /// `storage`'s Institutions ports under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.institutions.clone(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}

/// What the Institutions page's Router is built from for each call (its `PageView::Setup`): the
/// ports and Dialog keys its Dialogs open on.
pub struct InstitutionsRoutes {
    pub ports: lib_institutions::SharedPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}
