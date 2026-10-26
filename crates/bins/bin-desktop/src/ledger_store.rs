//! The Desktop's one copy of the Ledger (ADR-0038). [`LedgerStore`] holds the only [`Ledger`]: a
//! write runs through [`LedgerStore::apply`], and a landed one emits a [`LedgerChanged`] naming the
//! domains it touched, which [`notify_on`] turns into a repaint for the Entities reading them.
//!
//! The Ledger's Institutions, Accounts, Payees, Transactions, Bill Plans and Bill Schedule are read
//! copies of their ports (#774, #831, #849, #850, #853): `apply` forwards a command's Account,
//! Payee, Transaction and Bills changes to the ports and refreshes the copies from them. The
//! Institutions, Payees and Bills ports are shared with their modules, which write them through
//! their Interactors and refresh the copy after each event ([`LedgerStore::refresh_institutions`],
//! [`LedgerStore::refresh_payees`], [`LedgerStore::refresh_bills`]).
//!
//! The store holds no commit: what `apply` forwards is committed by the input it ran in
//! (`crate::input::send`). A View not yet a module runs `apply` from its own click, outside any
//! input, so the store then emits [`LegacyApplied`], which the root Router turns into one `send`
//! that commits it: `send`'s legacy arm, gone when the last such View migrates.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_accounts::AccountUse;
use lib_bills::BillsPorts as _;
use lib_core::AccountId;
use lib_database::{PayeesStore, SharedAccountsStore, SharedTransactionsStore};
use lib_institutions::InstitutionsPorts as _;
use lib_ledger::{Domain, Ledger, LedgerCommand, domains};

use crate::bills::ports::{DesktopPorts as DesktopBillsPorts, LedgerBackedPorts};
use crate::institutions::ports::DesktopPorts as DesktopInstitutionsPorts;
use crate::payees::ports::DesktopPorts as DesktopPayeesPorts;

/// What `shell::router::build` hands the Ledger: the Institutions ports it shares with the
/// Institutions module, the Accounts store it shares with the Units and Bills reads, the Payees
/// ports it shares with the Payees module and the Bills reads, where its Transactions are kept,
/// shared with the Bill Payments module, and the Bills ports it shares with the Bills module.
pub struct LedgerSetup {
    pub institutions: DesktopInstitutionsPorts,
    pub accounts: SharedAccountsStore,
    pub payees: DesktopPayeesPorts,
    pub transactions: SharedTransactionsStore,
    pub bills: DesktopBillsPorts,
}

impl LedgerSetup {
    /// The stub Ledger's Institutions, Accounts, Payees, Transactions and Bills as of `today`, in
    /// memory, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(today: chrono::NaiveDate) -> Self {
        let ledger = Ledger::stub_on(today);
        let payees = crate::payees::LedgerBackedPorts::in_memory(&ledger);
        let accounts = crate::settings::accounts::ports::in_memory(&ledger);
        Self {
            institutions: crate::institutions::LedgerBackedPorts::in_memory(&ledger),
            transactions: Rc::new(RefCell::new(lib_transactions::fake::InMemoryTransactions {
                transactions: ledger.transactions().to_vec(),
            })),
            bills: LedgerBackedPorts::in_memory(&ledger, payees.clone(), accounts.clone()),
            accounts,
            payees,
        }
    }
}

/// Emitted after a write lands, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

/// Emitted when [`LedgerStore::apply`] forwarded writes outside any input, so the root Router
/// sends one to commit them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyApplied;

pub struct LedgerStore {
    ledger: Ledger,
    institutions: DesktopInstitutionsPorts,
    accounts: SharedAccountsStore,
    payees: DesktopPayeesPorts,
    transactions: SharedTransactionsStore,
    bills: DesktopBillsPorts,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}
impl EventEmitter<LegacyApplied> for LedgerStore {}

impl LedgerStore {
    /// `ledger` with its own Institutions, Accounts, Payees, Transactions and Bills in memory as
    /// the ports, for a test that needs no storage.
    pub fn new(ledger: Ledger) -> Self {
        let transactions = lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        };
        let payees = crate::payees::LedgerBackedPorts::in_memory(&ledger);
        let accounts = crate::settings::accounts::ports::in_memory(&ledger);
        let bills = LedgerBackedPorts::in_memory(&ledger, payees.clone(), accounts.clone());
        Self {
            institutions: crate::institutions::LedgerBackedPorts::in_memory(&ledger),
            accounts,
            ledger,
            payees,
            transactions: Rc::new(RefCell::new(transactions)),
            bills,
        }
    }

    /// `ledger` with its Institutions, Accounts, Payees, Transactions and Bills read from `setup`'s
    /// stores, which its writes then go to.
    pub fn with_storage(ledger: Ledger, setup: LedgerSetup) -> Self {
        let mut store = Self {
            ledger,
            institutions: setup.institutions,
            accounts: setup.accounts,
            payees: setup.payees,
            transactions: setup.transactions,
            bills: setup.bills,
        };
        store.copy_institutions();
        store.copy_accounts();
        store.copy_payees();
        store.copy_transactions();
        store.copy_bills();
        store
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// What names Account `id`, as its Edit and Delete Dialogs show: the Transactions counted by
    /// the Accounts store, the Budgets by the Ledger until Budgets persist (#811).
    pub fn account_usage(&self, id: AccountId) -> AccountUse {
        crate::settings::accounts::ports::usage(&self.accounts, &self.ledger, id)
    }

    /// Leaves no Unit in use, so a test can delete a seeded one: the Accounts store's rows too,
    /// since the Units count their Accounts from it.
    #[doc(hidden)]
    pub fn drop_unit_users_for_test(&mut self, cx: &mut Context<'_, Self>) {
        self.ledger.drop_unit_users_for_test();
        let mut accounts = self.accounts.borrow_mut();
        for account in accounts.accounts().accounts() {
            accounts.accounts_mut().delete_account(account.id);
        }
        drop(accounts);
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

    /// Runs any command, the one write path a Dialog confirms through (ADR-0037). A refusal changes
    /// nothing and emits nothing; a landed command announces the domains its recorded changes
    /// touched. Its Account, Payee, Transaction and Bills changes go to the ports (the forwarding
    /// shims, #774, #831, #853), with the Unmatch a deleted Transaction's entries need, for the
    /// input's one commit; run outside any input, it emits [`LegacyApplied`] to be committed. The
    /// Account upserts and the Payees go first, since the Splits an Import books name the Payees it
    /// creates, and the Account deletes last, after the Transactions booked to them. The other
    /// domains' changes aren't persisted yet (#636), so they're dropped here.
    pub fn apply<C: LedgerCommand>(
        &mut self,
        cx: &mut Context<'_, Self>,
        command: C,
    ) -> Result<C::Outcome, C::Refusal> {
        let outcome = command.run(&mut self.ledger)?;
        let changes = self.ledger.drain_changes();
        if !changes.is_empty() {
            let accounts_upserted = crate::settings::accounts::ports::forward(
                &mut *self.accounts.borrow_mut(),
                &changes,
            );
            let payees_written =
                crate::payees::ports::forward(&mut *self.payees.borrow_mut(), &changes);
            crate::transactions::ports::forward(&mut *self.transactions.borrow_mut(), &changes);
            let bills_written = crate::bills::ports::forward(&self.bills, &changes);
            let accounts_deleted = crate::settings::accounts::ports::forward_deletes(
                &mut *self.accounts.borrow_mut(),
                &changes,
            );
            if !crate::input::sending(cx) {
                cx.emit(LegacyApplied);
            }
            let mut what = domains(&changes);
            if accounts_upserted || accounts_deleted {
                self.copy_accounts();
            }
            if payees_written {
                self.copy_payees();
            }
            if what.contains(&Domain::Transactions) {
                self.copy_transactions();
            }
            if bills_written && self.copy_bills() {
                what.insert(Domain::Bills);
            }
            self.announce(cx, what);
        }
        Ok(outcome)
    }

    /// Puts the Institutions back as stored, after a write that named them failed, and overwrites
    /// the read copy, announcing `Domain::Institutions` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the port and the read copy are left as they were.
    pub fn reload_institutions(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.institutions.borrow_mut().reload()?;
        self.copy_institutions();
        self.announce(cx, BTreeSet::from([Domain::Institutions]));
        Ok(())
    }

    /// Puts the Accounts back as stored, after a write that named them failed, and overwrites the
    /// read copy, announcing `Domain::Accounts` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the store and the read copy are left as they were.
    pub fn reload_accounts(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.accounts.borrow_mut().reload()?;
        self.copy_accounts();
        self.announce(cx, BTreeSet::from([Domain::Accounts]));
        Ok(())
    }

    /// Puts the Payees back as stored, after a write that named them failed, and overwrites the
    /// read copy, announcing `Domain::Payees` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the port and the read copy are left as they were.
    pub fn reload_payees(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.payees.borrow_mut().reload()?;
        self.copy_payees();
        self.announce(cx, BTreeSet::from([Domain::Payees]));
        Ok(())
    }

    /// Puts the Transactions back as stored, after a write that named them failed, and overwrites
    /// the read copy, announcing `Domain::Transactions` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the port and the read copy are left as they were.
    pub fn reload_transactions(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.transactions.borrow_mut().reload()?;
        self.copy_transactions();
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

    /// Brings the Institutions read copy in line with the ports after the Institutions module wrote
    /// them, announcing `Domain::Institutions` only when it changed.
    pub fn refresh_institutions(&mut self, cx: &mut Context<'_, Self>) {
        if self.copy_institutions() {
            self.announce(cx, BTreeSet::from([Domain::Institutions]));
        }
    }

    /// Brings the Payees read copy in line with the ports after the Payees module wrote them,
    /// announcing `Domain::Payees` only when it changed.
    pub fn refresh_payees(&mut self, cx: &mut Context<'_, Self>) {
        if self.copy_payees() {
            self.announce(cx, BTreeSet::from([Domain::Payees]));
        }
    }

    /// Brings the Bills read copy in line with the ports after the Bills module wrote them,
    /// announcing `Domain::Bills` only when it changed.
    pub fn refresh_bills(&mut self, cx: &mut Context<'_, Self>) {
        if self.copy_bills() {
            self.announce(cx, BTreeSet::from([Domain::Bills]));
        }
    }

    /// Brings the Transactions read copy in line with the port after the Bill Payments module
    /// wrote it, announcing `Domain::Transactions` only when it changed.
    pub fn refresh_transactions(&mut self, cx: &mut Context<'_, Self>) {
        if self.copy_transactions() {
            self.announce(cx, BTreeSet::from([Domain::Transactions]));
        }
    }

    /// Brings the Institutions read copy in line with the port, returning whether it changed.
    fn copy_institutions(&mut self) -> bool {
        let institutions = self.institutions.borrow().institutions().institutions();
        self.ledger.refresh_institutions(institutions)
    }

    /// Brings the Accounts read copy in line with the store, returning whether it changed.
    fn copy_accounts(&mut self) -> bool {
        let accounts = self.accounts.borrow().accounts().accounts();
        self.ledger.refresh_accounts(accounts)
    }

    /// Brings the Payees read copy in line with the port, returning whether it changed.
    fn copy_payees(&mut self) -> bool {
        let payees = PayeesStore::payees(&*self.payees.borrow()).payees();
        self.ledger.refresh_payees(payees)
    }

    /// Brings the read copy in line with the port, returning whether it changed.
    fn copy_transactions(&mut self) -> bool {
        let transactions = self.transactions.borrow().all_transactions();
        self.ledger.refresh_transactions(transactions)
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
    use lib_database::{Commit, PendingWrites};
    use lib_ledger::{LedgerParts, RemoveTag};

    use super::*;
    use lib_core::TagId;

    fn store(cx: &mut TestAppContext) -> (Entity<LedgerStore>, Rc<RefCell<Vec<LedgerChanged>>>) {
        let store = cx.new(|_| {
            LedgerStore::new(Ledger::from_parts(LedgerParts {
                tags: lib_tags::default_tags(),
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
        let id = store.read_with(cx, |store, _| store.ledger().tags()[0].id);

        store.update(cx, |store, cx| {
            store.apply(cx, RemoveTag(id)).expect("the Tag exists")
        });
        cx.run_until_parked();

        assert_eq!(
            *events.borrow(),
            vec![LedgerChanged {
                what: BTreeSet::from([Domain::Tags])
            }]
        );
    }

    #[gpui::test]
    fn a_refused_command_emits_nothing(cx: &mut TestAppContext) {
        let (store, events) = store(cx);

        store.update(cx, |store, cx| {
            assert!(store.apply(cx, RemoveTag(TagId::seed(u32::MAX))).is_err());
        });
        cx.run_until_parked();

        assert!(events.borrow().is_empty());
    }

    #[gpui::test]
    fn a_write_outside_any_input_asks_for_one_and_a_write_inside_one_asks_nothing(
        cx: &mut TestAppContext,
    ) {
        let (store, _) = store(cx);
        let asked = Rc::new(RefCell::new(0));
        let sink = asked.clone();
        cx.update(|cx| {
            crate::input::install(
                Rc::new(lib_database::InMemoryCommit),
                gpui::WeakEntity::new_invalid(),
                cx,
            );
            cx.subscribe(&store, move |_, _: &LegacyApplied, _| {
                *sink.borrow_mut() += 1;
            })
            .detach();
        });
        let ids: Vec<_> = store.read_with(cx, |store, _| {
            store.ledger().tags().iter().map(|tag| tag.id).collect()
        });

        store.update(cx, |store, cx| {
            store.apply(cx, RemoveTag(ids[0])).expect("removed")
        });
        cx.run_until_parked();
        assert_eq!(
            *asked.borrow(),
            1,
            "a View's own click asks the root Router for an input"
        );

        cx.update(|cx| {
            crate::input::send(cx, |cx| {
                store.update(cx, |store, cx| {
                    store.apply(cx, RemoveTag(ids[1])).expect("removed")
                })
            })
        });
        cx.run_until_parked();
        assert_eq!(*asked.borrow(), 1, "the input it ran in commits it");
    }

    /// The Desktop's start-up storage on a Ledger file in `dir`, as of `today`.
    fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> lib_database::Storage {
        lib_database::open_storage(&dir.join("ledger.pldb"), today).expect("the Ledger opens")
    }

    /// A store on `storage`, with the pending writes the test commits, as an input's `send` would.
    fn on_storage(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (Entity<LedgerStore>, PendingWrites) {
        let (store, _, _, pending) = on_storage_with_bills(cx, storage);
        (store, pending)
    }

    /// [`on_storage`] with the Bills ports the store shares, which the Bills module writes, and
    /// the payments port over both stores, which the Bill Payments module writes.
    fn on_storage_with_bills(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (
        Entity<LedgerStore>,
        DesktopBillsPorts,
        lib_bill_payments::SharedPayments,
        PendingWrites,
    ) {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let pending = storage.pending;
        let institutions =
            crate::institutions::LedgerBackedPorts::shared(Box::new(storage.institutions));
        let payees = crate::payees::LedgerBackedPorts::shared(Box::new(storage.payees));
        let accounts: SharedAccountsStore = Rc::new(RefCell::new(storage.accounts));
        let bills = LedgerBackedPorts::shared(
            Rc::new(RefCell::new(storage.bills)),
            payees.clone(),
            accounts.clone(),
        );
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(storage.transactions));
        let payments = crate::bill_payments::payments(transactions.clone(), &bills);
        let store = cx.new(|_| {
            LedgerStore::with_storage(
                Ledger::stub_on(today),
                LedgerSetup {
                    institutions,
                    accounts,
                    payees,
                    transactions,
                    bills: bills.clone(),
                },
            )
        });
        (store, bills, payments, pending)
    }

    #[gpui::test]
    fn an_import_and_a_deleted_accounts_transactions_survive_a_restart(cx: &mut TestAppContext) {
        use lib_ledger::{BookingPayee, CommitImport, DeleteAccount, ImportBooking};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let (store, pending) = on_storage(cx, storage);
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
        pending.commit();
        let before = store.read_with(cx, |store, _| store.ledger().transactions().to_vec());
        crate::quit(&writer);
        drop(store);
        drop(writer);

        // The next day, so a re-seed would show as shifted dates.
        let (reopened, _) = on_storage(
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

    /// Commits what was written, as the input's `send` would, then opens the Ledger in `dir`
    /// again a day later, so a re-seed would show as shifted dates.
    fn restart(
        cx: &mut TestAppContext,
        (store, pending): (Entity<LedgerStore>, PendingWrites),
        writer: lib_database::Writer,
        dir: &std::path::Path,
        today: chrono::NaiveDate,
    ) -> Entity<LedgerStore> {
        pending.commit();
        crate::quit(&writer);
        drop(store);
        drop(writer);
        on_storage(cx, storage_in(dir, today.succ_opt().expect("a date"))).0
    }

    #[gpui::test]
    fn paying_matching_skipping_and_editing_a_plan_survive_a_restart(cx: &mut TestAppContext) {
        use lib_bill_payments::interactor::PayBill;
        use lib_bills::interactor::{EditBillPlan, MatchBill, SkipBill};
        use lib_bills::{BillCategories as _, BillPlanDraft, Resolution};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let (store, bills, payments, pending) = on_storage_with_bills(cx, storage);
        let (plans, entries) = bills_of(cx, &store);
        let bill_accounts = bills.borrow().accounts().bill_accounts();
        let (matched, split) = store.read_with(cx, |store, _| {
            let ledger = store.ledger();
            entries
                .iter()
                .filter(|entry| entry.is_open())
                .find_map(|entry| {
                    lib_bills::match_candidates(
                        ledger.bill_plans(),
                        ledger.bill_entries(),
                        &ledger.bill_splits(),
                        &bill_accounts,
                        &ledger.bill_categories(),
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

        // Pay is the Bill Payments module's, through its port; Match, Skip and the edit are the
        // Bills module's, through its Interactors. One commit lands them all.
        PayBill {
            entry: paid,
            amount: "42.50".parse().expect("a decimal"),
            date: today,
        }
        .run(&mut *payments.borrow_mut())
        .expect("the entry is paid");
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
        cx.update(|cx| crate::bill_payments::refresh(&bills, &store, cx));
        let before = bills_of(cx, &store);
        let transactions = store.read_with(cx, |store, _| store.ledger().transactions().to_vec());

        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

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
        let (store, bills, _, pending) = on_storage_with_bills(cx, storage);
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

        // The file refuses deleting an Account a Bill Plan names (#853), so its Plans move to
        // another Account first: what's tested is the Unmatch.
        let other = store.read_with(cx, |store, _| {
            store
                .ledger()
                .accounts()
                .iter()
                .find(|a| a.id != account)
                .expect("a second Account")
                .id
        });
        {
            let mut ports = bills.borrow_mut();
            let plans = ports.bills().bill_plans();
            for mut plan in plans.into_iter().filter(|plan| plan.account_id == account) {
                plan.account_id = other;
                ports.split_mut().bills.upsert_bill_plan(plan);
            }
        }
        cx.update(|cx| crate::bills::ports::refresh(&bills, &store, cx));

        store.update(cx, |store, cx| {
            store
                .apply(cx, DeleteAccount(account))
                .expect("the Account is deleted");
        });
        assert_eq!(resolution(cx, &store), Some(Resolution::Unresolved));

        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        assert_eq!(resolution(cx, &reopened), Some(Resolution::Unresolved));
    }

    /// The halfway state until `lib-account-deletion` (#854): the file refuses deleting an Account
    /// a Bill Plan names, so the whole input is lost and a restart finds the Account and its
    /// Transactions as they were.
    #[gpui::test]
    fn deleting_an_account_a_bill_plan_names_is_refused_by_the_file(cx: &mut TestAppContext) {
        use lib_ledger::DeleteAccount;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let (store, pending) = on_storage(cx, storage);
        let (account, booked) = store.read_with(cx, |store, _| {
            let ledger = store.ledger();
            let plan = &ledger.bill_plans()[0];
            let booked = ledger
                .transactions()
                .iter()
                .filter(|t| t.account_id == plan.account_id)
                .count();
            (plan.account_id, booked)
        });
        assert!(booked > 0, "the stub books to a Bill Plan's Account");

        store.update(cx, |store, cx| {
            store
                .apply(cx, DeleteAccount(account))
                .expect("the Ledger deletes it");
        });

        let reopened = restart(cx, (store, pending), writer, dir.path(), today);
        reopened.read_with(cx, |store, _| {
            let ledger = store.ledger();
            assert!(ledger.accounts().iter().any(|a| a.id == account));
            assert_eq!(
                ledger
                    .transactions()
                    .iter()
                    .filter(|t| t.account_id == account)
                    .count(),
                booked
            );
        });
    }
}
