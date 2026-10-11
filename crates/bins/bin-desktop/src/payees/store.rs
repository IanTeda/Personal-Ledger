//! The shared Payees store (ADR-0032). [`PayeesStore`] owns the rows, read through `lib_payees`'
//! service.

use gpui::{Context, EventEmitter};
use lib_payees::{Payee, PayeeDraft, PayeeError, PayeeService};
use lib_transactions::Transaction;

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
    service: PayeeService,
}

impl EventEmitter<PayeesEvent> for PayeesStore {}

impl PayeesStore {
    /// A store over `payees`, which the Desktop seeds from `lib_payees::default_payees`.
    pub fn new(payees: Vec<Payee>) -> Self {
        Self {
            service: PayeeService::new(payees),
        }
    }

    pub fn payees(&self) -> &[Payee] {
        self.service.payees()
    }

    /// Adds a Payee and selects nothing; the caller decides what to select.
    pub fn insert(
        &mut self,
        cx: &mut Context<'_, Self>,
        draft: &PayeeDraft,
    ) -> Result<u32, PayeeError> {
        let id = self.service.insert(draft)?;
        cx.emit(PayeesEvent::Changed);
        Ok(id)
    }

    pub fn edit(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        draft: &PayeeDraft,
    ) -> Result<(), PayeeError> {
        self.service.edit(id, draft)?;
        cx.emit(PayeesEvent::Changed);
        Ok(())
    }

    /// Applies the Delete dialog's `action` to Payee `id`. `transactions` is what the rules check
    /// a delete against, so the caller passes the Transactions store's rows.
    pub fn apply_delete(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        action: DeleteAction,
        transactions: &[Transaction],
    ) -> Result<(), PayeeError> {
        match action {
            DeleteAction::Delete => self.service.delete(id, transactions),
            DeleteAction::Deactivate => self.service.set_active(id, false),
            DeleteAction::Reactivate => self.service.set_active(id, true),
        }?;
        cx.emit(PayeesEvent::Changed);
        Ok(())
    }

    /// Replaces every row with `rows`, a copy the caller has already run through the rules (see
    /// [`PayeeService::replace`]).
    pub fn replace(&mut self, cx: &mut Context<'_, Self>, rows: Vec<Payee>) {
        self.service.replace(rows);
        cx.emit(PayeesEvent::Changed);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};

    use super::*;

    /// A `PayeesStore` over the seeded Payees, with every event it emits collected.
    fn seeded_store(
        cx: &mut TestAppContext,
    ) -> (Entity<PayeesStore>, Rc<RefCell<Vec<PayeesEvent>>>) {
        let store = cx.new(|_| PayeesStore::new(lib_payees::default_payees()));
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
        let taken = store.read_with(cx, |store, _| store.payees()[0].name.clone());

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
        let mut rows = store.read_with(cx, |store, _| store.payees().to_vec());
        rows.truncate(1);

        store.update(cx, |store, cx| store.replace(cx, rows));
        cx.run_until_parked();

        assert_eq!(*events.borrow(), vec![PayeesEvent::Changed]);
        assert_eq!(store.read_with(cx, |store, _| store.payees().len()), 1);
    }
}
