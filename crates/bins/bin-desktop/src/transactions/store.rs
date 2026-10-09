//! The shared Transactions store (ADR-0032). [`TransactionsStore`] owns the Transactions, read
//! through `lib_transactions`' service. The Transactions page, Tags, Payees, Accounts, Bills and
//! Documents read the rows from here. `Shell` does not observe the store, so a write reaches the
//! screen on the next render only when its caller also notifies `Shell`, as the call sites do.

use gpui::{App, Context, Entity};
use lib_transactions::{Transaction, TransactionService};

pub struct TransactionsStore {
    service: TransactionService,
}

impl TransactionsStore {
    pub fn new(transactions: Vec<Transaction>) -> Self {
        Self {
            service: TransactionService::from_rows(transactions),
        }
    }

    pub fn transactions(&self) -> &[Transaction] {
        self.service.transactions()
    }

    /// Applies `change` to the rows, then tells every subscriber they changed. Mutations go through
    /// here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<Transaction>) -> R,
    ) -> R {
        let result = self.service.edit(change);
        cx.notify();
        result
    }
}

/// Edits the Transactions through their store from a caller that holds no other Entity's borrow.
/// Use it where a closure needs to reach other `Shell` fields, which `TransactionsStore::mutate`
/// cannot, since it borrows the store itself.
pub fn edit_transactions<R>(
    store: &Entity<TransactionsStore>,
    cx: &mut App,
    change: impl FnOnce(&mut Vec<Transaction>) -> R,
) -> R {
    store.update(cx, |store, cx| store.mutate(cx, change))
}
