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
use lib_account_deletion::SharedDeletion;
use lib_database::{
    AccountDeletionAdapter, SharedAccountsStore, SharedTransactionsStore, StoredAccountReads,
};

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

/// What the Delete Dialog reads and writes through: the deletion port, the Bills ports whose
/// reads a confirm copies back, and the figures it quotes as it opens: the Transactions counted by
/// the Transactions store, the Budgets by the Budgets store and the Unit codes read from the Units
/// store (#952, #1052).
#[derive(Clone)]
pub struct DeletionStores {
    pub deletion: SharedDeletion,
    pub bills: DesktopPorts,
    pub reads: StoredAccountReads,
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
            reads: StoredAccountReads {
                transactions: storage.transactions.clone(),
                budgets: storage.budgets.clone(),
                units: storage.units.clone(),
            },
        }
    }
}

/// Brings the Ledger's Accounts read copy in line after a confirm and announces the Transactions
/// and the Bills, so the Accounts page, the Transactions View, the Dashboard and the Bills page
/// see the delete at once. The confirm's input commits it.
pub(crate) fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| {
        store.refresh_accounts(cx);
        store.announce_transactions(cx);
    });
    crate::bills::ports::announce(ledger, cx);
}
