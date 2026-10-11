//! The shared Bills store (ADR-0032). [`BillsStore`] reads the Bill Plans and the Bill Schedule
//! through [`LedgerStore`], which holds the only copy. The Bills page, the Dashboard, Budgets and
//! Documents read them from here. `Shell` does not observe the store, so a write reaches the screen
//! on the next render only when its caller also notifies `Shell`, as the call sites do.

use gpui::{App, Context, Entity, Subscription};
use lib_bills::{BillPlan, BillScheduleEntry};
use lib_ledger::{Domain, LedgerHatch};
use lib_transactions::Transaction;

use crate::ledger_store::{LedgerChanged, LedgerStore};

pub struct BillsStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl BillsStore {
    /// A store over `ledger`'s Bill Plans and Bill Schedule.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Bills) {
                cx.notify();
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn plans<'a>(&self, cx: &'a App) -> &'a [BillPlan] {
        self.ledger.read(cx).ledger().bill_plans()
    }

    pub fn entries<'a>(&self, cx: &'a App) -> &'a [BillScheduleEntry] {
        self.ledger.read(cx).ledger().bill_entries()
    }

    /// Applies `change` to the Plans and entries, then tells every subscriber they changed.
    /// Mutations go through here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut App,
        change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>) -> R,
    ) -> R {
        self.write(cx, &[Domain::Bills], |ledger| {
            change(ledger.bill_plans, ledger.bill_entries)
        })
    }

    #[expect(deprecated, reason = "Bills writes move onto Ledger commands in #643")]
    fn write<R>(
        &self,
        cx: &mut App,
        what: &[Domain],
        change: impl FnOnce(LedgerHatch<'_>) -> R,
    ) -> R {
        self.ledger
            .update(cx, |ledger, cx| ledger.write(cx, what, change))
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
/// it resolves an entry. One write to the Ledger, announcing both.
pub fn edit_bills_and_transactions<R>(
    bills: &Entity<BillsStore>,
    cx: &mut App,
    change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>, &mut Vec<Transaction>) -> R,
) -> R {
    bills.update(cx, |store, cx| {
        store.write(cx, &[Domain::Bills, Domain::Transactions], |ledger| {
            change(ledger.bill_plans, ledger.bill_entries, ledger.transactions)
        })
    })
}
