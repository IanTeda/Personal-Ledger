//! The Desktop's one copy of the Ledger (ADR-0038). [`LedgerStore`] holds the only [`Ledger`]: a
//! write runs through [`LedgerStore::apply`], and a landed one emits a [`LedgerChanged`] naming the
//! domains it touched, which [`notify_on`] turns into a repaint for the Entities reading them.
//!
//! The Ledger's Transactions are a read copy of the Transactions port (#774): `apply` forwards a
//! command's Transaction changes to the port, commits, and refreshes the copy from it.

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_database::{Commit, TransactionsStore};
use lib_ledger::{Domain, Ledger, LedgerCommand, domains};

use crate::transactions::ports::{TransactionsSetup, forward};

/// Emitted after a write lands, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

pub struct LedgerStore {
    ledger: Ledger,
    transactions: Box<dyn TransactionsStore>,
    commit: Rc<dyn Commit>,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}

impl LedgerStore {
    /// `ledger` with its own Transactions in memory as the port, for a test that needs no
    /// storage.
    pub fn new(ledger: Ledger) -> Self {
        let transactions = lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        };
        Self {
            ledger,
            transactions: Box::new(transactions),
            commit: Rc::new(lib_database::InMemoryCommit),
        }
    }

    /// `ledger` with its Transactions read from `setup`'s store, which its writes then go to.
    pub fn with_storage(mut ledger: Ledger, setup: TransactionsSetup) -> Self {
        ledger.refresh_transactions(setup.store.transactions().transactions());
        Self {
            ledger,
            transactions: setup.store,
            commit: setup.commit,
        }
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
    /// nothing; a landed command announces the domains its recorded changes touched. Its
    /// Transaction changes are saved as one commit (the forwarding shim, #774); the other domains'
    /// aren't persisted yet (#636), so they're dropped here.
    pub fn apply<C: LedgerCommand>(
        &mut self,
        cx: &mut Context<'_, Self>,
        command: C,
    ) -> Result<C::Outcome, C::Refusal> {
        let outcome = command.run(&mut self.ledger)?;
        let changes = self.ledger.drain_changes();
        if !changes.is_empty() {
            forward(self.transactions.as_mut(), &changes);
            self.commit.commit();
            let what = domains(&changes);
            if what.contains(&Domain::Transactions) {
                self.refresh_transactions();
            }
            self.announce(cx, what);
        }
        Ok(outcome)
    }

    /// Puts the Transactions back as stored, after a write that named them failed, and overwrites
    /// the read copy, announcing `Domain::Transactions` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the port and the read copy are left as they were.
    pub fn reload_transactions(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.transactions.reload()?;
        self.refresh_transactions();
        self.announce(cx, BTreeSet::from([Domain::Transactions]));
        Ok(())
    }

    /// Brings the read copy in line with the port.
    fn refresh_transactions(&mut self) {
        self.ledger
            .refresh_transactions(self.transactions.transactions().transactions());
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

    /// The Desktop's start-up storage on a Ledger file in `dir`, as of `today`.
    fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> lib_database::Storage {
        lib_database::open_storage(&dir.join("ledger.pldb"), today).expect("the Ledger opens")
    }

    fn on_storage(cx: &mut TestAppContext, storage: lib_database::Storage) -> Entity<LedgerStore> {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        cx.new(|_| {
            LedgerStore::with_storage(
                Ledger::stub_on(today),
                TransactionsSetup {
                    store: Box::new(storage.transactions),
                    commit: Rc::new(storage.pending),
                },
            )
        })
    }

    #[gpui::test]
    fn an_import_and_a_deleted_accounts_transactions_survive_a_restart(cx: &mut TestAppContext) {
        use lib_ledger::{BookingPayee, CommitImport, DeleteAccount, ImportBooking};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let store = on_storage(cx, storage);
        let (wallet, everyday, groceries) = store.read_with(cx, |store, _| {
            let ledger = store.ledger();
            let account = |name: &str| {
                ledger
                    .accounts()
                    .iter()
                    .find(|account| account.name == name)
                    .expect("a stub Account")
                    .id
            };
            let groceries = lib_categories::find_by_name(ledger.categories(), "Groceries")
                .expect("a stub Category");
            (account("Wallet"), account("ANZ Everyday"), groceries)
        });

        store.update(cx, |store, cx| {
            store
                .apply(
                    cx,
                    CommitImport {
                        account_id: everyday,
                        bookings: vec![ImportBooking {
                            date: today,
                            description: "IMPORTED SHOP".into(),
                            amount: "-12.34".parse().expect("a decimal"),
                            payee: BookingPayee::Create {
                                name: "Corner Store".into(),
                                alias: None,
                            },
                            category_id: groceries,
                        }],
                    },
                )
                .expect("the import books");
            store
                .apply(cx, DeleteAccount(wallet))
                .expect("the Wallet is deleted");
        });
        let before = store.read_with(cx, |store, _| store.ledger().transactions().to_vec());
        crate::quit(&writer);
        drop(store);
        drop(writer);

        // The next day, so a re-seed would show as shifted dates.
        let reopened = on_storage(
            cx,
            storage_in(dir.path(), today.succ_opt().expect("a date")),
        );
        let after = reopened.read_with(cx, |store, _| store.ledger().transactions().to_vec());

        assert_eq!(after, before);
        assert!(
            after
                .iter()
                .all(|transaction| transaction.account_id != wallet)
        );
        assert!(
            after
                .iter()
                .any(|transaction| transaction.description.as_deref() == Some("IMPORTED SHOP"))
        );
    }
}
