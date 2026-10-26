//! The ports Accounts reach storage and the other domains through, in the shape of `lib-payees'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! The page's Presenters and its Router trait arrive with the page (#855). Until then the Desktop
//! writes Accounts through `LedgerCommand`s, and `lib-ledger` answers [`AccountUsage`] from the
//! `Ledger`.

use lib_core::AccountId;

use crate::Account;

/// The Accounts on the Ledger.
pub trait AccountRepository {
    /// Every Account, in the order they were added.
    fn accounts(&self) -> Vec<Account>;

    fn account(&self, id: AccountId) -> Option<Account> {
        self.accounts().into_iter().find(|account| account.id == id)
    }

    /// Writes `account` over the row with its id, or as a new row after every row already there.
    fn upsert_account(&mut self, account: Account);

    /// Removes Account `id`'s row. The Transactions booked to it are not this port's to remove.
    fn delete_account(&mut self, id: AccountId);
}

/// How many rows of the other domains name one Account, as its Edit and Delete Dialogs show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccountUse {
    /// The Transactions booked to it, counted from the Transactions themselves.
    pub transactions: usize,
    /// The Budgets that count it.
    pub budgets: usize,
}

/// The read the Edit and Delete Dialogs count an Account's Transactions and Budgets through, so
/// `lib-accounts` needn't depend on either domain's crate. Only a figure the Dialogs show, so a
/// read that can't be made is the answerer's to report, not a refusal here.
pub trait AccountUsage {
    /// What names Account `id`; an Account nothing names, or one that doesn't exist, has none.
    fn account_usage(&self, id: AccountId) -> AccountUse;
}
