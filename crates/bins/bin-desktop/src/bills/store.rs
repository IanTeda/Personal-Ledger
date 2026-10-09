//! The shared Bills store (ADR-0032). [`BillsStore`] owns the Bill Plans and the Bill Schedule,
//! read through `lib_bills`' service. The Bills page, the Dashboard, Budgets and Documents read
//! them from here. `Shell` does not observe the store, so a write reaches the screen on the next
//! render only when its caller also notifies `Shell`, as the call sites do.

use gpui::{App, Context, Entity};
use lib_bills::{BillPlan, BillScheduleEntry, BillService};
use lib_transactions::Transaction;

use crate::transactions::TransactionsStore;

pub struct BillsStore {
    service: BillService,
}

impl BillsStore {
    pub fn new(plans: Vec<BillPlan>, entries: Vec<BillScheduleEntry>) -> Self {
        Self {
            service: BillService::from_rows(plans, entries),
        }
    }

    pub fn plans(&self) -> &[BillPlan] {
        self.service.plans()
    }

    pub fn entries(&self) -> &[BillScheduleEntry] {
        self.service.entries()
    }

    /// Applies `change` to the Plans and entries, then tells every subscriber they changed.
    /// Mutations go through here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>) -> R,
    ) -> R {
        let result = self.service.edit(change);
        cx.notify();
        result
    }
}

/// Edits the Bills through their store from a caller that holds no other Entity's borrow. Use it
/// where a closure needs to reach other `Shell` fields, which `BillsStore::mutate` cannot, since it
/// borrows the store itself.
pub fn edit_bills<R>(
    store: &Entity<BillsStore>,
    cx: &mut App,
    change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>) -> R,
) -> R {
    store.update(cx, |store, cx| store.mutate(cx, change))
}

/// Edits the Bills and the Transactions together, for a settle that writes or reads a Split while
/// it resolves an entry. Both stores notify.
pub fn edit_bills_and_transactions<R>(
    bills: &Entity<BillsStore>,
    transactions: &Entity<TransactionsStore>,
    cx: &mut App,
    change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>, &mut Vec<Transaction>) -> R,
) -> R {
    bills.update(cx, |store, cx| {
        let result = transactions.update(cx, |rows, cx| {
            rows.mutate(cx, |rows| {
                store
                    .service
                    .edit(|plans, entries| change(plans, entries, rows))
            })
        });
        cx.notify();
        result
    })
}
