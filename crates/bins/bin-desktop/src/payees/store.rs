//! The shared Payees store (ADR-0032). [`PayeesStore`] reads the rows through [`LedgerStore`],
//! which holds the only copy, and re-emits [`PayeesEvent::Changed`] when a write touches them.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::{Domain, LedgerHatch};
use lib_payees::{
    Payee, PayeeDraft, PayeeError, delete_payee, edit_payee, insert_payee, set_active,
};
use lib_transactions::Transaction;

use crate::ledger_store::{LedgerChanged, LedgerStore};
use crate::payees::form::DeleteAction;

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

    /// Adds a Payee and selects nothing; the caller decides what to select.
    pub fn insert(&mut self, cx: &mut App, draft: &PayeeDraft) -> Result<u32, PayeeError> {
        self.try_write(cx, |ledger| insert_payee(ledger.payees, draft))
    }

    pub fn edit(&mut self, cx: &mut App, id: u32, draft: &PayeeDraft) -> Result<(), PayeeError> {
        self.try_write(cx, |ledger| edit_payee(ledger.payees, id, draft))
    }

    /// Applies the Delete dialog's `action` to Payee `id`. `transactions` is what the rules check
    /// a delete against, so the caller passes the Transactions store's rows.
    pub fn apply_delete(
        &mut self,
        cx: &mut App,
        id: u32,
        action: DeleteAction,
        transactions: &[Transaction],
    ) -> Result<(), PayeeError> {
        self.try_write(cx, |ledger| match action {
            DeleteAction::Delete => delete_payee(ledger.payees, transactions, id),
            DeleteAction::Deactivate => set_active(ledger.payees, id, false),
            DeleteAction::Reactivate => set_active(ledger.payees, id, true),
        })
    }

    /// Replaces every row with `rows`, a copy the caller has already run through the rules. Import's
    /// commit is the only caller, since it also writes Transactions.
    pub fn replace(&mut self, cx: &mut App, rows: Vec<Payee>) {
        self.write(cx, |ledger| *ledger.payees = rows);
    }

    #[expect(deprecated, reason = "Payees writes move onto Ledger commands in #638")]
    fn write<R>(&self, cx: &mut App, change: impl FnOnce(LedgerHatch<'_>) -> R) -> R {
        self.ledger
            .update(cx, |ledger, cx| ledger.write(cx, &[Domain::Payees], change))
    }

    #[expect(deprecated, reason = "Payees writes move onto Ledger commands in #638")]
    fn try_write<T>(
        &self,
        cx: &mut App,
        change: impl FnOnce(LedgerHatch<'_>) -> Result<T, PayeeError>,
    ) -> Result<T, PayeeError> {
        self.ledger.update(cx, |ledger, cx| {
            ledger.try_write(cx, &[Domain::Payees], change)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_ledger::{Ledger, LedgerParts};

    use super::*;

    /// A `PayeesStore` over the seeded Payees, with every event it emits collected.
    fn seeded_store(
        cx: &mut TestAppContext,
    ) -> (Entity<PayeesStore>, Rc<RefCell<Vec<PayeesEvent>>>) {
        let ledger = cx.new(|_| {
            LedgerStore::new(Ledger::from_parts(LedgerParts {
                payees: lib_payees::default_payees(),
                ..LedgerParts::default()
            }))
        });
        let store = cx.new(|cx| PayeesStore::new(ledger, cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&store, move |_, event: &PayeesEvent, _| {
                sink.borrow_mut().push(*event)
            })
            .detach();
        });
        (store, events)
    }

    fn draft(name: &str) -> PayeeDraft {
        PayeeDraft {
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
        }
    }

    #[gpui::test]
    fn a_landed_write_emits_changed_and_a_refused_one_does_not(cx: &mut TestAppContext) {
        let (store, events) = seeded_store(cx);
        let taken = store.read_with(cx, |store, cx| store.payees(cx)[0].name.clone());

        store.update(cx, |store, cx| {
            assert!(store.insert(cx, &draft(&taken)).is_err());
        });
        cx.run_until_parked();
        assert!(events.borrow().is_empty());

        store.update(cx, |store, cx| {
            store
                .insert(cx, &draft("Corner Shop"))
                .expect("a free name is accepted");
        });
        cx.run_until_parked();
        assert_eq!(*events.borrow(), vec![PayeesEvent::Changed]);
    }

    #[gpui::test]
    fn replace_emits_changed_so_every_reader_sees_the_copy(cx: &mut TestAppContext) {
        let (store, events) = seeded_store(cx);
        let mut rows = store.read_with(cx, |store, cx| store.payees(cx).to_vec());
        rows.truncate(1);

        store.update(cx, |store, cx| store.replace(cx, rows));
        cx.run_until_parked();

        assert_eq!(*events.borrow(), vec![PayeesEvent::Changed]);
        assert_eq!(store.read_with(cx, |store, cx| store.payees(cx).len()), 1);
    }
}
