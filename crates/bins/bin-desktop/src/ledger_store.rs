//! The Desktop's one copy of the Ledger (ADR-0038). [`LedgerStore`] holds the only [`Ledger`]: a
//! write runs through [`LedgerStore::apply`], and a landed one emits a [`LedgerChanged`] naming the
//! domains it touched, which [`notify_on`] turns into a repaint for the Entities reading them.
//!
//! The Ledger's Transactions, Bill Plans and Bill Schedule are read copies of their ports (#774):
//! `apply` forwards a command's Transaction and Bills changes to the ports, commits, and refreshes
//! the copies from them. The Bills ports are shared with the Bills module, which writes them
//! through its Interactors and refreshes the copy after each event ([`LedgerStore::refresh_bills`]).

use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_bills::BillsPorts as _;
use lib_database::{Commit, TransactionsStore};
use lib_ledger::{Domain, Ledger, LedgerCommand, domains};

use crate::bills::ports::{DesktopPorts as DesktopBillsPorts, LedgerBackedPorts};

/// What `build_shell` hands the Ledger: where its Transactions are kept, the Bills ports it
/// shares with the Bills module, and the commit that ends each call's writes, every adapter's at
/// once.
pub struct LedgerSetup {
    pub transactions: Box<dyn TransactionsStore>,
    pub bills: DesktopBillsPorts,
    pub commit: Rc<dyn Commit>,
}

impl LedgerSetup {
    /// The stub Ledger's Transactions and Bills as of `today`, in memory, for a test that builds
    /// `Shell` directly.
    #[cfg(test)]
    pub fn stub(today: chrono::NaiveDate) -> Self {
        let ledger = Ledger::stub_on(today);
        Self {
            transactions: Box::new(lib_transactions::fake::InMemoryTransactions {
                transactions: ledger.transactions().to_vec(),
            }),
            bills: LedgerBackedPorts::in_memory(&ledger),
            commit: Rc::new(lib_database::InMemoryCommit),
        }
    }
}

/// Emitted after a write lands, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

pub struct LedgerStore {
    ledger: Ledger,
    transactions: Box<dyn TransactionsStore>,
    bills: DesktopBillsPorts,
    commit: Rc<dyn Commit>,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}

impl LedgerStore {
    /// `ledger` with its own Transactions and Bills in memory as the ports, for a test that needs
    /// no storage.
    pub fn new(ledger: Ledger) -> Self {
        let transactions = lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        };
        let bills = LedgerBackedPorts::in_memory(&ledger);
        Self {
            ledger,
            transactions: Box::new(transactions),
            bills,
            commit: Rc::new(lib_database::InMemoryCommit),
        }
    }

    /// `ledger` with its Transactions and Bills read from `setup`'s stores, which its writes then
    /// go to.
    pub fn with_storage(ledger: Ledger, setup: LedgerSetup) -> Self {
        let mut store = Self {
            ledger,
            transactions: setup.transactions,
            bills: setup.bills,
            commit: setup.commit,
        };
        store.refresh_transactions();
        store.copy_bills();
        store
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
    /// Transaction and Bills changes are saved as one commit (the forwarding shims, #774), with
    /// the Unmatch a deleted Transaction's entries need; the other domains' aren't persisted yet
    /// (#636), so they're dropped here.
    pub fn apply<C: LedgerCommand>(
        &mut self,
        cx: &mut Context<'_, Self>,
        command: C,
    ) -> Result<C::Outcome, C::Refusal> {
        let outcome = command.run(&mut self.ledger)?;
        let changes = self.ledger.drain_changes();
        if !changes.is_empty() {
            crate::transactions::ports::forward(self.transactions.as_mut(), &changes);
            let bills_written = crate::bills::ports::forward(&self.bills, &changes);
            self.commit.commit();
            let mut what = domains(&changes);
            if what.contains(&Domain::Transactions) {
                self.refresh_transactions();
            }
            if bills_written && self.copy_bills() {
                what.insert(Domain::Bills);
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

    /// Puts the Bill Plans and the Bill Schedule back as stored, after a write that named them
    /// failed, and overwrites the read copy, announcing `Domain::Bills` so every reader redraws
    /// (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the ports and the read copy are left as they were.
    pub fn reload_bills(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.bills.borrow_mut().reload()?;
        self.copy_bills();
        self.announce(cx, BTreeSet::from([Domain::Bills]));
        Ok(())
    }

    /// Brings the Bills read copy in line with the ports after the Bills module wrote them,
    /// announcing `Domain::Bills` only when it changed.
    pub fn refresh_bills(&mut self, cx: &mut Context<'_, Self>) {
        if self.copy_bills() {
            self.announce(cx, BTreeSet::from([Domain::Bills]));
        }
    }

    /// Brings the read copy in line with the port.
    fn refresh_transactions(&mut self) {
        self.ledger
            .refresh_transactions(self.transactions.transactions().transactions());
    }

    /// Brings the Bills read copy in line with the ports, returning whether it changed.
    fn copy_bills(&mut self) -> bool {
        let (plans, entries) = {
            let ports = self.bills.borrow();
            (ports.bills().bill_plans(), ports.bills().entries())
        };
        self.ledger.refresh_bills(plans, entries)
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
        on_storage_with_bills(cx, storage).0
    }

    /// [`on_storage`] with the Bills ports the store shares, which the Bills module writes.
    fn on_storage_with_bills(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (Entity<LedgerStore>, DesktopBillsPorts) {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let commit: Rc<dyn Commit> = Rc::new(storage.pending);
        let bills = LedgerBackedPorts::shared(Box::new(storage.bills), commit.clone());
        let store = cx.new(|_| {
            LedgerStore::with_storage(
                Ledger::stub_on(today),
                LedgerSetup {
                    transactions: Box::new(storage.transactions),
                    bills: bills.clone(),
                    commit,
                },
            )
        });
        (store, bills)
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

    /// The Bills read copy, as every reader takes it.
    fn bills_of(
        cx: &mut TestAppContext,
        store: &Entity<LedgerStore>,
    ) -> (Vec<lib_bills::BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
        store.read_with(cx, |store, _| {
            (
                store.ledger().bill_plans().to_vec(),
                store.ledger().bill_entries().to_vec(),
            )
        })
    }

    /// Writes nothing more, then opens the Ledger in `dir` again a day later, so a re-seed would
    /// show as shifted dates.
    fn restart(
        cx: &mut TestAppContext,
        store: Entity<LedgerStore>,
        writer: lib_database::Writer,
        dir: &std::path::Path,
        today: chrono::NaiveDate,
    ) -> Entity<LedgerStore> {
        crate::quit(&writer);
        drop(store);
        drop(writer);
        on_storage(cx, storage_in(dir, today.succ_opt().expect("a date")))
    }

    #[gpui::test]
    fn paying_matching_skipping_and_editing_a_plan_survive_a_restart(cx: &mut TestAppContext) {
        use lib_bills::interactor::{EditBillPlan, MatchBill, SkipBill};
        use lib_bills::{BillPlanDraft, Resolution};
        use lib_ledger::PayBill;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let (store, bills) = on_storage_with_bills(cx, storage);
        let (plans, entries) = bills_of(cx, &store);
        let (matched, split) = store.read_with(cx, |store, _| {
            let ledger = store.ledger();
            entries
                .iter()
                .filter(|entry| entry.is_open())
                .find_map(|entry| {
                    lib_bills::match_candidates(
                        ledger.bill_plans(),
                        ledger.bill_entries(),
                        ledger.transactions(),
                        ledger.accounts(),
                        ledger.categories(),
                        entry.id(),
                    )
                    .first()
                    .map(|split| (entry.id(), *split))
                })
                .expect("the stub leaves an entry with a Match candidate")
        });
        let mut open = entries
            .iter()
            .filter(|entry| entry.is_open() && entry.id() != matched)
            .map(lib_bills::BillScheduleEntry::id);
        let paid = open.next().expect("a second open entry");
        let skipped = open
            .find(|entry| entry.plan_id != paid.plan_id)
            .expect("a third open entry");
        let plan = plans[0].clone();

        store.update(cx, |store, cx| {
            store
                .apply(
                    cx,
                    PayBill {
                        entry: paid,
                        amount: "42.50".parse().expect("a decimal"),
                        date: today,
                    },
                )
                .expect("the entry is paid");
        });
        // Match, Skip and the edit are the Bills module's, through its Interactors.
        cx.update(|cx| crate::bills::ports::copy_reads(&bills, store.read(cx).ledger()));
        {
            let mut ports = bills.borrow_mut();
            let ports = ports.split_mut();
            MatchBill {
                entry: matched,
                split,
            }
            .run(
                ports.bills,
                ports.transactions,
                ports.accounts,
                ports.categories,
            )
            .expect("the entry is matched");
            SkipBill { entry: skipped }
                .run(ports.bills)
                .expect("the entry is skipped");
            EditBillPlan {
                id: plan.id,
                draft: BillPlanDraft {
                    name: "Renamed plan".into(),
                    category_id: plan.category_id,
                    unit: plan.unit,
                    account_id: plan.account_id,
                    payee_id: plan.payee_id,
                    planned_amount: plan.planned_amount.clone(),
                    amount_kind: plan.amount_kind,
                    recurrence: plan.recurrence,
                    first_due: plan.first_due,
                    ends_on: plan.ends_on,
                    attention_lead: plan.attention_lead,
                },
                is_active: plan.is_active,
                today,
            }
            .run(ports.bills, ports.accounts, ports.categories)
            .expect("the plan is edited");
        }
        cx.update(|cx| crate::bills::ports::sync(&bills, &store, cx));
        let before = bills_of(cx, &store);
        let transactions = store.read_with(cx, |store, _| store.ledger().transactions().to_vec());

        let reopened = restart(cx, store, writer, dir.path(), today);

        assert_eq!(bills_of(cx, &reopened), before);
        let after = &before.1;
        let resolution = |id| lib_bills::entry(after, id).map(|entry| entry.resolution.clone());
        assert_eq!(resolution(matched), Some(Resolution::Paid(split)));
        assert!(matches!(
            resolution(skipped),
            Some(Resolution::Skipped { .. })
        ));
        let Some(Resolution::Paid(payment)) = resolution(paid) else {
            panic!("the paid entry stays Paid");
        };
        assert!(
            reopened.read_with(cx, |store, _| store
                .ledger()
                .transactions()
                .iter()
                .any(|t| t.id == payment.transaction_id)),
            "Pay's Transaction is stored with it"
        );
        assert_eq!(
            reopened.read_with(cx, |store, _| store.ledger().transactions().to_vec()),
            transactions
        );
        assert_eq!(before.0[0].name, "Renamed plan");
    }

    #[gpui::test]
    fn deleting_an_account_leaves_the_entries_its_transactions_paid_unmatched_after_a_restart(
        cx: &mut TestAppContext,
    ) {
        use lib_bills::Resolution;
        use lib_ledger::DeleteAccount;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let store = on_storage(cx, storage);
        let (entry, account) = store.read_with(cx, |store, _| {
            let ledger = store.ledger();
            ledger
                .bill_entries()
                .iter()
                .find_map(|entry| match entry.resolution {
                    Resolution::Paid(split) => ledger
                        .transactions()
                        .iter()
                        .find(|t| t.id == split.transaction_id)
                        .map(|t| (entry.id(), t.account_id)),
                    _ => None,
                })
                .expect("the stub pays an entry")
        });
        let resolution = |cx: &mut TestAppContext, store: &Entity<LedgerStore>| {
            store.read_with(cx, |store, _| {
                lib_bills::entry(store.ledger().bill_entries(), entry)
                    .map(|entry| entry.resolution.clone())
            })
        };

        store.update(cx, |store, cx| {
            store
                .apply(cx, DeleteAccount(account))
                .expect("the Account is deleted");
        });
        assert_eq!(resolution(cx, &store), Some(Resolution::Unresolved));

        let reopened = restart(cx, store, writer, dir.path(), today);

        assert_eq!(resolution(cx, &reopened), Some(Resolution::Unresolved));
    }
}
