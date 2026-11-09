//! The Accounts module's Desktop half (`docs/development/viper-architecture.md`): the Views for
//! the Accounts Settings page and its Add and Edit Dialogs, and `impl AccountsRouter`. Every rule,
//! every piece of text and the keymap are `lib-accounts`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`AccountsPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are the form Presenter's `DialogView`, hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`accounts_page`]). It opens
//!   `lib-account-deletion`'s Delete Dialog through that module's entry point.
//!
//! The ports are `lib-database`'s `StoredAccountsPorts`, which `LedgerPorts` hands the page and its
//! Dialogs (#1084).

pub(crate) mod router;
mod view;

pub use router::accounts_page;

use lib_accounts::presenter::AccountsPagePresenter;
use lib_core::InstitutionId;
use lib_institutions::Institution;

use crate::shell::view::module_page::ModulePage;

/// The Accounts Settings page: the page Presenter in the generic page Entity.
pub type AccountsPage = ModulePage<AccountsPagePresenter>;

/// What to show for an account's `institution`: the name of the one of `institutions` it names,
/// unchanged (Ledger data, not a Message). An id `institutions` doesn't hold shows nothing.
pub fn institution_label(institution: InstitutionId, institutions: &[Institution]) -> String {
    lib_institutions::get(institutions, institution)
        .map_or_else(String::new, |institution| institution.name.clone())
}

/// The Accounts page: names in the order the page lists them, and the selected row's name
/// (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAccountsSnapshot {
    pub account_names: Vec<String>,
    pub selected_account: Option<String>,
}

/// What `shell::router::build` hands the Accounts module: the page's Presenter, which owns the
/// Accounts keymap under `[keybindings]`, its ports, what its Delete Dialog reads and writes
/// through, and the Dialog keys every Accounts Dialog's hints read.
pub struct AccountsSetup {
    pub presenter: AccountsPagePresenter,
    pub ports: lib_accounts::SharedPorts,
    pub deletion: crate::account_deletion::DeletionStores,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl AccountsSetup {
    pub fn new(
        ports: lib_accounts::SharedPorts,
        deletion: crate::account_deletion::DeletionStores,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: AccountsPagePresenter::new(
                ports.clone(),
                lib_accounts::keymap::accounts_page_keymap(config),
            ),
            ports,
            deletion,
            dialog_keys,
        }
    }

    /// `storage`'s Accounts ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.accounts.clone(),
            crate::account_deletion::DeletionStores::of(&storage.ports),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}

/// What the Accounts page's Router is built from for each call (its `PageView::Setup`): the ports,
/// the Delete Dialog's stores and the Dialog keys its Dialogs open on, and the Clock an Added
/// Account's opening date and the Transactions hand-off read.
pub struct AccountsRoutes {
    pub ports: lib_accounts::SharedPorts,
    pub deletion: crate::account_deletion::DeletionStores,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    pub clock: lib_core::Clock,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn institution_label_names_an_institution_by_its_id() {
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            let institutions = lib_institutions::sample_institutions();
            assert_eq!(
                institution_label(InstitutionId::SEED_PERSONAL, &institutions),
                "Personal"
            );
            assert_eq!(
                institution_label(InstitutionId::SEED_ANZ, &institutions),
                "ANZ Banking Group"
            );
            assert_eq!(institution_label(InstitutionId::new(), &institutions), "");
        });
    }
}
