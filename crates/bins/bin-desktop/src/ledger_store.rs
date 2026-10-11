//! The Desktop's one copy of the Ledger (ADR-0038). [`LedgerStore`] holds the only [`Ledger`]; the
//! per-domain stores hold none of their own rows, reading through it and re-emitting their own
//! event when a [`LedgerChanged`] names their domain, so the Views reading them don't change yet.

use std::collections::BTreeSet;

use gpui::{Context, EventEmitter};
use lib_ledger::{Domain, Ledger, LedgerCommand, LedgerHatch, domains};

/// Emitted after a write lands, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

pub struct LedgerStore {
    ledger: Ledger,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}

impl LedgerStore {
    pub fn new(ledger: Ledger) -> Self {
        Self { ledger }
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Runs any command: the Dialog host's `Apply` (ADR-0037). A refusal changes nothing and emits
    /// nothing; a landed command announces the domains its recorded changes touched. Nothing
    /// persists the changes until the repository is wired (#636), so they're dropped here.
    pub fn apply<C: LedgerCommand>(
        &mut self,
        cx: &mut Context<'_, Self>,
        command: C,
    ) -> Result<C::Outcome, C::Refusal> {
        let outcome = command.run(&mut self.ledger)?;
        let changes = self.ledger.drain_changes();
        if !changes.is_empty() {
            self.announce(cx, domains(&changes));
        }
        Ok(outcome)
    }

    /// Writes through the Ledger's hatch and announces `what`, which the caller names because a
    /// hatch write records no changes to derive it from.
    #[deprecated(note = "a write that skips the change list; move it onto a Ledger command")]
    pub fn write<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        what: &[Domain],
        change: impl FnOnce(LedgerHatch<'_>) -> R,
    ) -> R {
        #[expect(deprecated, reason = "the store's own hatch forwards to the Ledger's")]
        let result = change(self.ledger.hatch());
        self.announce(cx, what.iter().copied().collect());
        result
    }

    /// [`Self::write`] for a write that can be refused: announces `what` only when it lands, so a
    /// refusal emits nothing.
    #[deprecated(note = "a write that skips the change list; move it onto a Ledger command")]
    pub fn try_write<T, E>(
        &mut self,
        cx: &mut Context<'_, Self>,
        what: &[Domain],
        change: impl FnOnce(LedgerHatch<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        #[expect(deprecated, reason = "the store's own hatch forwards to the Ledger's")]
        let result = change(self.ledger.hatch());
        if result.is_ok() {
            self.announce(cx, what.iter().copied().collect());
        }
        result
    }

    fn announce(&mut self, cx: &mut Context<'_, Self>, what: BTreeSet<Domain>) {
        cx.emit(LedgerChanged { what });
        cx.notify();
    }
}

/// A `LedgerStore` over `parts`, for a test that needs only some collections.
#[cfg(test)]
pub(crate) fn test_ledger(
    cx: &mut gpui::TestAppContext,
    parts: lib_ledger::LedgerParts,
) -> gpui::Entity<LedgerStore> {
    use gpui::AppContext;
    cx.new(|_| LedgerStore::new(Ledger::from_parts(parts)))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_ledger::{DeletePayee, LedgerParts};

    use super::*;

    fn store(cx: &mut TestAppContext) -> (Entity<LedgerStore>, Rc<RefCell<Vec<LedgerChanged>>>) {
        let store = cx.new(|_| {
            LedgerStore::new(Ledger::from_parts(LedgerParts {
                payees: lib_payees::default_payees(),
                ..LedgerParts::default()
            }))
        });
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&store, move |_, event: &LedgerChanged, _| {
                sink.borrow_mut().push(event.clone())
            })
            .detach();
        });
        (store, events)
    }

    #[gpui::test]
    fn a_landed_command_announces_the_domains_it_changed(cx: &mut TestAppContext) {
        let (store, events) = store(cx);
        let id = store.read_with(cx, |store, _| store.ledger().payees()[0].id);

        store.update(cx, |store, cx| {
            store
                .apply(cx, DeletePayee(id))
                .expect("the Payee exists and nothing names it")
        });
        cx.run_until_parked();

        assert_eq!(
            *events.borrow(),
            vec![LedgerChanged {
                what: BTreeSet::from([Domain::Payees])
            }]
        );
    }

    #[gpui::test]
    fn a_refused_command_emits_nothing(cx: &mut TestAppContext) {
        let (store, events) = store(cx);

        store.update(cx, |store, cx| {
            assert!(store.apply(cx, DeletePayee(u32::MAX)).is_err());
        });
        cx.run_until_parked();

        assert!(events.borrow().is_empty());
    }

    #[gpui::test]
    #[expect(deprecated, reason = "tests the hatch itself")]
    fn a_refused_hatch_write_emits_nothing_and_a_landed_one_names_its_domains(
        cx: &mut TestAppContext,
    ) {
        let (store, events) = store(cx);

        store.update(cx, |store, cx| {
            let refused: Result<(), ()> = store.try_write(cx, &[Domain::Payees], |_| Err(()));
            assert!(refused.is_err());
        });
        cx.run_until_parked();
        assert!(events.borrow().is_empty());

        store.update(cx, |store, cx| {
            store.write(cx, &[Domain::Bills, Domain::Transactions], |_| ())
        });
        cx.run_until_parked();
        assert_eq!(
            *events.borrow(),
            vec![LedgerChanged {
                what: BTreeSet::from([Domain::Transactions, Domain::Bills])
            }]
        );
    }
}
