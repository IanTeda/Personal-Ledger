//! The Desktop's announcer of Ledger writes (#1057). [`Announcer`] holds no rows, no stores and no
//! commit: every domain is written by its module, through its ports or a cross-domain module's
//! port, over the stores the Open Ledger assembles, and the module then announces the [`Domain`]
//! it wrote. The announcement is a [`LedgerChanged`], which [`notify_on`] turns into a repaint for
//! the Entities reading that domain, and which the pages that filter on domains (the Dashboard,
//! Bills, Budgets, Documents, Transactions) read themselves.
//!
//! A module that can't tell from an event whether it wrote keeps its own write count, on its ports
//! (the Units, Accounts, Institutions and Categories ports, for instance), and announces only when
//! it moved: the Announcer keeps no count of anyone's writes. The input that wrote commits the writes (`crate::input::send`), not the Announcer.

use std::collections::BTreeSet;

use gpui::{Context, Entity, EventEmitter, Subscription};

/// A part of the Ledger a write can touch: what [`LedgerChanged`] carries when a module has
/// written its store, so a reader can skip a change that isn't its own. Distinct from
/// `lib_database`'s writer `Domain`, which names the adapters a failed write reloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Accounts,
    Categories,
    Payees,
    Tags,
    Transactions,
    Bills,
    Budgets,
    Documents,
    DocumentTypes,
    Inventory,
    Institutions,
    Units,
}

/// Emitted after a write lands, naming the domain it touched. "Ledger" is the glossary's Ledger,
/// the person's data, not a type.
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
    /// Tells every reader of `domain` that it changed.
    pub fn announce(&mut self, domain: Domain, cx: &mut Context<'_, Self>) {
        self.announcements += 1;
        cx.emit(LedgerChanged {
            what: BTreeSet::from([domain]),
        });
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
