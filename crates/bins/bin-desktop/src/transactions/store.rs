//! The shared Transactions store (ADR-0032). [`TransactionsStore`] reads the Transactions through
//! [`LedgerStore`], which holds the only copy. The Transactions page, Tags, Payees, Accounts, Bills
//! and Documents read the rows from here. `Shell` does not observe the store, so a write reaches the
//! screen on the next render only when its caller also notifies `Shell`, as the call sites do.

use gpui::{App, Context, Entity, Subscription};
use lib_ledger::Domain;
use lib_transactions::Transaction;

use crate::ledger_store::{LedgerChanged, LedgerStore};

pub struct TransactionsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl TransactionsStore {
    /// A store over `ledger`'s Transactions.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Transactions) {
                cx.notify();
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.ledger.read(cx).ledger().transactions()
    }

    /// Applies `change` to the rows, then tells every subscriber they changed. Mutations go through
    /// here so no write can skip the notification.
    #[expect(
        deprecated,
        reason = "Transactions writes move onto Ledger commands in #642"
    )]
    pub fn mutate<R>(
        &mut self,
        cx: &mut App,
        change: impl FnOnce(&mut Vec<Transaction>) -> R,
    ) -> R {
        self.ledger.update(cx, |ledger, cx| {
            ledger.write(cx, &[Domain::Transactions], |ledger| {
                change(ledger.transactions)
            })
        })
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
