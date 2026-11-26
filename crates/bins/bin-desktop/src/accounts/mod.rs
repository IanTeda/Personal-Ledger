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
//! The ports are `lib_accounts::SharedPorts`, which `LedgerPorts::accounts()` hands the page and
//! its Dialogs (#1084).

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

/// What the Accounts page's Router is built from for each call (its `PageView::Setup`): the ports,
/// what its Delete Dialog reads and writes through, the Dialog keys every Accounts Dialog's hints
/// read, and the Clock an Added Account's opening date and the Transactions hand-off read.
pub struct AccountsSetup {
    pub ports: lib_accounts::SharedPorts,
    pub deletion: lib_account_deletion::DeletionPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    pub clock: lib_core::Clock,
}

impl AccountsSetup {
    /// The page's Presenter, which owns the Accounts keymap under `[keybindings]`, and its Setup.
    pub fn new(
        ports: lib_accounts::SharedPorts,
        deletion: lib_account_deletion::DeletionPorts,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        clock: lib_core::Clock,
        config: &lib_core::configuration::KeyBindingConfig,
    ) -> (AccountsPagePresenter, Self) {
        let presenter = AccountsPagePresenter::new(ports.clone(), config);
        let setup = Self {
            ports,
            deletion,
            dialog_keys,
            clock,
        };
        (presenter, setup)
    }

    /// `ports`' Accounts ports under the default keys, dated by `clock`, for a test that builds
    /// `Root` directly.
    #[cfg(test)]
    pub fn sample(
        ports: &lib_database::LedgerPorts,
        clock: &lib_core::Clock,
    ) -> (AccountsPagePresenter, Self) {
        let config = lib_core::configuration::KeyBindingConfig::default();
        Self::new(
            ports.accounts(),
            ports.account_deletion(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            clock.clone(),
            &config,
        )
    }
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
