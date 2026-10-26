//! The Account Deletion module's Desktop half (`docs/development/viper-architecture.md`, #854): the
//! Delete account Dialog's View and the entry point the Accounts page opens it through. Every
//! rule, every piece of text and the Presenter are `lib-account-deletion`'s; this folder turns gpui
//! keys and clicks into Presenter events and draws the view model.
//!
//! - `view/dialog/delete_account.rs` holds the Presenter in an Entity in the Dialog host and draws
//!   it.
//! - `router.rs` is the module's entry point ([`router::open_delete`]), which the legacy Accounts
//!   page calls until the Accounts page is a module (#855).
//!
//! Account delete writes through [`deletion`], `lib-database`'s adapter over the Accounts and
//! Transactions stores `LedgerStore` holds and the Bills store the Bills ports hold, so the
//! Account, its Transactions and the Unmatch of the entries they paid join the one pending batch
//! and land in one commit.

pub mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_account_deletion::{DeletionReads, SharedDeletion};
use lib_accounts::{AccountUsage, AccountUse};
use lib_core::{AccountId, UnitId};
use lib_database::{AccountDeletionAdapter, SharedAccountsStore, SharedTransactionsStore};

use crate::bills::DesktopPorts;
use crate::ledger_store::LedgerStore;

pub use view::dialog::delete_account::DeleteAccountDialog;

/// The deletion port over `accounts`, `transactions` and the Bills store `bills` writes.
pub fn deletion(
    accounts: SharedAccountsStore,
    transactions: SharedTransactionsStore,
    bills: &DesktopPorts,
) -> SharedDeletion {
    Rc::new(RefCell::new(AccountDeletionAdapter::new(
        accounts,
        transactions,
        bills.borrow().store(),
    )))
}

/// What the Delete Dialog reads and writes through: the deletion port, and the Bills ports whose
/// reads a confirm copies back.
#[derive(Clone)]
pub struct DeletionStores {
    pub deletion: SharedDeletion,
    pub bills: DesktopPorts,
}

impl DeletionStores {
    /// The deletion port over `storage`'s stores, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self {
            deletion: deletion(
                storage.accounts.clone(),
                storage.transactions.clone(),
                &storage.bills,
            ),
            bills: storage.bills.clone(),
        }
    }
}

/// The figures the Delete Dialog quotes, read from the Ledger when it opens: the Transactions
/// counted by the Accounts store and the Budgets by the Ledger until Budgets persist (#811), and
/// the Unit codes from its Units read copy.
pub(crate) struct LedgerReads<'a>(pub &'a LedgerStore);

impl AccountUsage for LedgerReads<'_> {
    fn account_usage(&self, id: AccountId) -> AccountUse {
        self.0.account_usage(id)
    }
}

impl DeletionReads for LedgerReads<'_> {
    fn unit_code(&self, unit: UnitId) -> Option<String> {
        let code = self.0.ledger().unit_code(unit);
        (!code.is_empty()).then(|| code.to_string())
    }
}

/// Brings the Ledger's Accounts, Transactions and Bills read copies in line after a confirm, so
/// the Accounts page, the Transactions View, the Dashboard and the Bills page see the delete at
/// once, and copies the reads back into the Bills ports for the next event. The confirm's input
/// commits it.
pub(crate) fn refresh(bills: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| {
        store.refresh_accounts(cx);
        store.refresh_transactions(cx);
    });
    crate::bills::ports::refresh(bills, ledger, cx);
}
