//! The Accounts module's Desktop half (`docs/development/viper-architecture.md`): the Views for
//! the Accounts Settings page and its Add and Edit Dialogs, and `impl AccountsRouter`. Every rule,
//! every piece of text and the keymap are `lib-accounts`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds the Add and Edit Dialog's Presenter in an Entity in the Dialog host.
//! - `router.rs` is the Router and the module's entry point ([`accounts_page`]). It opens
//!   `lib-account-deletion`'s Delete Dialog through that module's entry point.
//! - `ports.rs` holds the halfway shims: the Institutions, Units and Budgets copied from the
//!   Ledger, and the Ledger read copy refreshed after each write (#855).
//!
//! Where the Accounts are kept, the SQLite adapter opened at start or memory, is `lib-database`'s
//! `AccountsStore`.

pub mod ports;
mod router;
mod view;

pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::accounts_page;
pub use view::page::AccountsPage;

use lib_accounts::NO_INSTITUTION;
use lib_accounts::presenter::AccountsPageEvent;
use lib_core::InstitutionId;
use lib_institutions::Institution;

/// What to show for an account's `institution`: the placeholder's Message in the Locale in
/// effect, or the name of the one of `institutions` it names, unchanged (Ledger data, not a
/// Message). An id `institutions` doesn't hold shows nothing.
pub fn institution_label(institution: InstitutionId, institutions: &[Institution]) -> String {
    if institution == NO_INSTITUTION {
        lib_locale::msg::institution_none()
    } else {
        lib_institutions::get(institutions, institution)
            .map_or_else(String::new, |institution| institution.name.clone())
    }
}

/// The Accounts page: names in the order the page lists them, and the selected row's name
/// (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAccountsSnapshot {
    pub account_names: Vec<String>,
    pub selected_account: Option<String>,
}

/// What `shell::router::build` hands the Accounts module: its ports, shared with the Ledger, the
/// Units and Bills reads and the deletion adapter, the stores its Delete Dialog writes through,
/// and its keymap under `[keybindings]`.
pub struct AccountsSetup {
    pub ports: DesktopPorts,
    pub deletion: crate::account_deletion::DeletionStores,
    pub keymap: lib_viper::keymap::Keymap<AccountsPageEvent>,
}

impl AccountsSetup {
    pub fn new(
        ports: DesktopPorts,
        deletion: crate::account_deletion::DeletionStores,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            ports,
            deletion,
            keymap: lib_accounts::keymap::accounts_page_keymap(config),
        }
    }

    /// `storage`'s Accounts ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.accounts.clone(),
            crate::account_deletion::DeletionStores::stub(storage),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn institution_label_renders_the_placeholder_and_names_an_institution_by_its_id() {
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            let institutions = lib_institutions::default_institutions();
            assert_eq!(
                institution_label(NO_INSTITUTION, &institutions),
                "No institution"
            );
            assert_eq!(
                institution_label(InstitutionId::SEED_ANZ, &institutions),
                "ANZ Banking Group"
            );
            assert_eq!(institution_label(InstitutionId::new(), &institutions), "");
        });
    }
}
