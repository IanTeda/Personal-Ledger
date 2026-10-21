//! The Desktop's one copy of the Ledger (ADR-0038). [`LedgerStore`] holds the only [`Ledger`]: a
//! write runs through [`LedgerStore::apply`], and a landed one emits a [`LedgerChanged`] naming the
//! domains it touched, which [`notify_on`] turns into a repaint for the Entities reading them.

use std::collections::BTreeSet;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_ledger::{Domain, Ledger, LedgerCommand, domains};

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

    /// Leaves no Unit in use, so a test can delete a seeded one.
    #[doc(hidden)]
    pub fn drop_unit_users_for_test(&mut self, cx: &mut Context<'_, Self>) {
        self.ledger.drop_unit_users_for_test();
        cx.emit(LedgerChanged {
            what: BTreeSet::from([
                Domain::Units,
                Domain::Accounts,
                Domain::Bills,
                Domain::Budgets,
            ]),
        });
        cx.notify();
    }

    /// Refreshes the Units read copy from the Units ports (#746's halfway shim), announcing
    /// `Domain::Units` only when it changed.
    pub fn refresh_units(
        &mut self,
        units: Vec<lib_units::UnitRow>,
        base_unit: lib_core::UnitId,
        price_sources: Vec<lib_units::PriceSourceRecord>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.ledger.refresh_units(units, base_unit, price_sources) {
            self.announce(cx, BTreeSet::from([Domain::Units]));
        }
    }

    /// Runs any command, the one write path a Dialog confirms through (ADR-0037). A refusal changes nothing and emits
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

    fn announce(&mut self, cx: &mut Context<'_, Self>, what: BTreeSet<Domain>) {
        cx.emit(LedgerChanged { what });
        cx.notify();
    }
}

/// Repaints the subscribing Entity whenever a landed write touches `domain`: how an Entity that
/// reads the Ledger straight from `ledger` follows the rows it shows (ADR-0038).
pub fn notify_on<T: 'static>(
    ledger: &Entity<LedgerStore>,
    domain: Domain,
    cx: &mut Context<'_, T>,
) -> Subscription {
    cx.subscribe(ledger, move |_, _, event: &LedgerChanged, cx| {
        if event.what.contains(&domain) {
            cx.notify();
        }
    })
}

/// A `LedgerStore` over `parts`, for a test that needs only some collections.
#[cfg(test)]
pub(crate) fn test_ledger(
    cx: &mut gpui::TestAppContext,
    parts: lib_ledger::LedgerParts,
) -> Entity<LedgerStore> {
    use gpui::AppContext;
    cx.new(|_| LedgerStore::new(Ledger::from_parts(parts)))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_ledger::{DeletePayee, LedgerParts};

    use super::*;
    use lib_core::PayeeId;

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
            assert!(
                store
                    .apply(cx, DeletePayee(PayeeId::seed(u32::MAX)))
                    .is_err()
            );
        });
        cx.run_until_parked();

        assert!(events.borrow().is_empty());
    }
}
