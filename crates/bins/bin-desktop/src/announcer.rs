//! The Desktop's announcer of Ledger writes (#1057, #1081). [`Announcer`] holds no rows, no stores
//! and no commit, and only the input door (`crate::input`) calls it: each input commits once, and
//! the [`Committed`](lib_database::Committed) it returns names every domain the input wrote or
//! cascaded into, which the door announces in one [`LedgerChanged`], with the domains a failed
//! write's rollback reloaded. No module counts or announces its own writes. [`notify_on`] turns the
//! announcement into a repaint for the Entities reading a domain, and the pages that filter on
//! domains (the Dashboard, Bills, Budgets, Documents, Transactions) read it themselves.

use std::collections::BTreeSet;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_database::Domain;

/// Emitted once after an input that changed the Ledger, naming every domain it touched. "Ledger"
/// is the glossary's Ledger, the person's data, not a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

/// The one emitter of [`LedgerChanged`]. Its count of announcements is how the Shell tells,
/// without a subscription that would run only after the input's update, whether anything was
/// announced since it last read the rail's counts.
#[derive(Default)]
pub struct Announcer {
    announcements: usize,
}

impl EventEmitter<LedgerChanged> for Announcer {}

impl Announcer {
    /// Tells every reader of a domain in `what` that it changed: the input door's, once per input
    /// that changed something.
    pub(crate) fn announce(&mut self, what: BTreeSet<Domain>, cx: &mut Context<'_, Self>) {
        self.announcements += 1;
        cx.emit(LedgerChanged { what });
        cx.notify();
    }

    /// How many changes were announced so far: a difference since the last look is a change.
    pub fn announcements(&self) -> usize {
        self.announcements
    }
}

/// Repaints the subscribing Entity whenever a landed write touches `domain`: how an Entity that
/// reads a domain through its stores follows the rows it shows.
pub fn notify_on<T: 'static>(
    announcer: &Entity<Announcer>,
    domain: Domain,
    cx: &mut Context<'_, T>,
) -> Subscription {
    cx.subscribe(announcer, move |_, _, event: &LedgerChanged, cx| {
        if event.what.contains(&domain) {
            cx.notify();
        }
    })
}
