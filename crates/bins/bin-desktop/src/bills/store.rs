//! The shared Bills store (ADR-0032). [`BillsStore`] reads the Bill Plans and the Bill Schedule
//! through [`LedgerStore`], which holds the only copy. The Bills page, the Dashboard, Budgets and
//! Documents read them from here. `Shell` does not observe the store, so a write reaches the screen
//! on the next render only when its caller also notifies `Shell`, as the call sites do.

use gpui::{App, Context, Entity, Subscription};
use lib_bills::{BillPlan, BillScheduleEntry};
use lib_ledger::Domain;

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
}
