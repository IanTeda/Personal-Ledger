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
}
