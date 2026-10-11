//! The shared Payees store (ADR-0032). [`PayeesStore`] is a read view over [`LedgerStore`], which
//! holds the only copy and takes every Payees write as a command, and re-emits
//! [`PayeesEvent::Changed`] when a write touches them.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_payees::Payee;

use crate::ledger_store::{LedgerChanged, LedgerStore};

/// Emitted by [`PayeesStore`] after a write lands, so `Shell` can refresh the Views that read the
/// rows (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayeesEvent {
    Changed,
}

/// The shared Payees rows. Every reader (the Payees page, Settings, the Transactions form and
/// Import, Bills, Documents and Budgets) reads them from here, so a change is seen everywhere on
/// the next render.
pub struct PayeesStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<PayeesEvent> for PayeesStore {}

impl PayeesStore {
    /// A store over `ledger`'s Payees.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Payees) {
                cx.emit(PayeesEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.ledger.read(cx).ledger().payees()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_ledger::{AddPayee, Ledger, LedgerParts};
    use lib_payees::PayeeDraft;

    use super::*;

    /// A `PayeesStore` over the seeded Payees, with every event it emits collected.
    fn seeded_store(
        cx: &mut TestAppContext,
    ) -> (
        Entity<LedgerStore>,
        Entity<PayeesStore>,
        Rc<RefCell<Vec<PayeesEvent>>>,
    ) {
        let ledger = cx.new(|_| {
            LedgerStore::new(Ledger::from_parts(LedgerParts {
                payees: lib_payees::default_payees(),
                ..LedgerParts::default()
            }))
        });
        let store = cx.new(|cx| PayeesStore::new(ledger.clone(), cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&store, move |_, event: &PayeesEvent, _| {
                sink.borrow_mut().push(*event)
            })
            .detach();
        });
        (ledger, store, events)
    }

    #[gpui::test]
    fn a_landed_command_emits_changed_and_a_refused_one_does_not(cx: &mut TestAppContext) {
        let (ledger, store, events) = seeded_store(cx);
        let taken = store.read_with(cx, |store, cx| store.payees(cx)[0].name.clone());
        let draft = |name: &str| PayeeDraft {
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
        };

        ledger.update(cx, |ledger, cx| {
            assert!(ledger.apply(cx, AddPayee(draft(&taken))).is_err());
        });
        cx.run_until_parked();
        assert!(events.borrow().is_empty());

        ledger.update(cx, |ledger, cx| {
            ledger
                .apply(cx, AddPayee(draft("Corner Shop")))
                .expect("a free name is accepted");
        });
        cx.run_until_parked();
        assert_eq!(*events.borrow(), vec![PayeesEvent::Changed]);
    }
}
