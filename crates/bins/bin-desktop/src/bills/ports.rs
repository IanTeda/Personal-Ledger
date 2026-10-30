//! The Bills ports as the Desktop holds them: the store `shell::router::build` is given, shared by
//! the Bills module and `LedgerStore`, the Payees store Bills reads its Payees from (#831), the
//! Accounts store it reads its Accounts from (#853), the Categories store it reads its
//! Categories from (#858) and the Transactions store it reads its Splits from (#926), with the
//! Units Bills reads copied from the Ledger (#778's halfway shim), and the step that keeps the
//! Ledger's read copy in line after a Bills write. The write is committed by the input it was made
//! in (`crate::input::send`), not here.
//!
//! The Units are copied before each Bills event, since a port can't reach the `LedgerStore`
//! Entity without a gpui context; they go when the Units answer through their own port.
//! `LedgerStore` writes the store too, through [`forward`]: the Bills writer not yet migrated,
//! Category reassign, still runs as a `LedgerCommand`. The store is shared with the Bill Payments
//! module too, whose Pay it directly writes an entry, and the Account Deletion module, whose delete
//! Unmatches the entries an Account's Transactions paid (#854).

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_bills::fake::InMemoryBillUnits;
use lib_bills::interactor::PopulateSchedule;
use lib_bills::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillSplit, BillTransactions, BillUnits, BillsPorts, BillsRepository,
    PortsMut, SplitRef,
};
use lib_database::{
    SharedAccountsStore, SharedBillsStore, SharedCategoriesStore, SharedPayeesStore,
    SharedTransactionsStore,
};
use lib_ledger::{Ledger, LedgerChange};

use crate::ledger_store::LedgerStore;

/// The Bills ports the Bills module and `LedgerStore` share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The store's two repositories as the one a Bills Interactor writes through. It forwards rather
/// than lending the store's own: `bills()` returns a borrow that must outlive the call, and the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
struct Store(SharedBillsStore);

impl BillPlanRepository for Store {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.0.borrow().plans().bill_plans()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        self.0.borrow_mut().plans_mut().upsert_bill_plan(plan);
    }
}

impl BillScheduleRepository for Store {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.0.borrow().entries().entries()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        self.0.borrow_mut().entries_mut().upsert_entry(entry);
    }
}

/// The Payees store as Bills reads it, borrowed for each read for the reason [`Store`] is.
struct Payees(SharedPayeesStore);

impl BillPayees for Payees {
    fn bill_payees(&self) -> Vec<lib_bills::BillPayee> {
        self.0.borrow().bill_payees()
    }
}

/// The Accounts store as Bills reads it, borrowed for each read for the reason [`Store`] is.
struct Accounts(SharedAccountsStore);

impl BillAccounts for Accounts {
    fn bill_accounts(&self) -> Vec<lib_bills::BillAccount> {
        self.0.borrow().bill_accounts()
    }
}

/// The Categories store as Bills reads it, borrowed for each read for the reason [`Store`] is.
struct Categories(SharedCategoriesStore);

impl BillCategories for Categories {
    fn bill_categories(&self) -> Vec<lib_bills::BillCategory> {
        self.0.borrow().bill_categories()
    }
}

/// The Transactions store as Bills reads it, borrowed for each read for the reason [`Store`] is.
struct Transactions(SharedTransactionsStore);

impl BillTransactions for Transactions {
    fn splits_between(&self, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Vec<BillSplit> {
        self.0.borrow().bill_splits_between(from, to)
    }

    fn paid_split(&self, split: SplitRef) -> Option<BillSplit> {
        self.0.borrow().bill_paid_split(split)
    }
}

/// `store`'s Bill Plans and Bill Schedule, `payees`' Payees, `accounts`' Accounts, `categories`'
/// Categories and `transactions`' Splits, with the Units copied from the Ledger.
pub struct LedgerBackedPorts {
    store: Store,
    payees: Payees,
    accounts: Accounts,
    categories: Categories,
    transactions: Transactions,
    units: InMemoryBillUnits,
}

impl LedgerBackedPorts {
    pub fn shared(
        store: SharedBillsStore,
        payees: SharedPayeesStore,
        accounts: SharedAccountsStore,
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store: Store(store),
            payees: Payees(payees),
            accounts: Accounts(accounts),
            categories: Categories(categories),
            transactions: Transactions(transactions),
            units: InMemoryBillUnits::default(),
        }))
    }

    /// `ledger`'s own Bill Plans and Bill Schedule in memory, reading `payees`, `accounts`,
    /// `categories` and `transactions`, for a test that needs no storage.
    pub fn in_memory(
        ledger: &Ledger,
        payees: SharedPayeesStore,
        accounts: SharedAccountsStore,
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
    ) -> DesktopPorts {
        let bills = Rc::new(RefCell::new(lib_bills::fake::InMemoryBills {
            plans: ledger.bill_plans().to_vec(),
            entries: ledger.bill_entries().to_vec(),
        }));
        let ports = Self::shared(bills, payees, accounts, categories, transactions);
        ports.borrow_mut().copy_reads(ledger);
        ports
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    pub fn reload(&mut self) -> lib_database::Result<()> {
        self.store.0.borrow_mut().reload()
    }

    /// The Bills store the ports write, which the Bill Payments module writes too.
    pub fn store(&self) -> SharedBillsStore {
        self.store.0.clone()
    }

    /// Copies the Units Bills reads from `ledger`, so the next Interactor and view model read
    /// them.
    fn copy_reads(&mut self, ledger: &Ledger) {
        self.units = InMemoryBillUnits {
            codes: ledger
                .units()
                .iter()
                .map(|unit| (unit.id, unit.code.clone()))
                .collect(),
            base: ledger.base_unit_row().map(|unit| unit.id),
        };
    }
}

impl BillsPorts for LedgerBackedPorts {
    fn bills(&self) -> &dyn BillsRepository {
        &self.store
    }

    fn transactions(&self) -> &dyn BillTransactions {
        &self.transactions
    }

    fn accounts(&self) -> &dyn BillAccounts {
        &self.accounts
    }

    fn categories(&self) -> &dyn BillCategories {
        &self.categories
    }

    fn payees(&self) -> &dyn BillPayees {
        &self.payees
    }

    fn units(&self) -> &dyn BillUnits {
        &self.units
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            bills: &mut self.store,
            transactions: &self.transactions,
            accounts: &self.accounts,
            categories: &self.categories,
        }
    }
}

/// Sends each Bill Plan and entry in `changes` to the store, in the order recorded. The writer it
/// carries: Category reassign. Returns whether it wrote anything, so the caller refreshes the read
/// copy.
pub fn forward(ports: &DesktopPorts, changes: &[LedgerChange]) -> bool {
    let mut ports = ports.borrow_mut();
    let store = &mut ports.store;
    let mut wrote = false;
    for change in changes {
        match change {
            LedgerChange::UpsertBillPlan(plan) => {
                store.upsert_bill_plan(plan.clone());
                wrote = true;
            }
            LedgerChange::UpsertBillEntry(entry) => {
                store.upsert_entry(entry.clone());
                wrote = true;
            }
            _ => {}
        }
    }
    wrote
}

/// Brings the stored Bill Schedule up to the horizon as of `today` (ADR-0019's "on Client start"),
/// writing the entries generation adds. The caller commits them.
pub fn generate(store: &mut dyn BillsRepository, today: chrono::NaiveDate) {
    PopulateSchedule { today }.run(store);
}

/// Brings the Bill Schedule up to the horizon for the month a running Desktop has crossed into,
/// wherever it is (ADR-0019), then refreshes the read copy as after a Bills event, so the
/// Dashboard, the rail badge and Budgets see the new entries. The day poll's input commits them as
/// one write, and a failed save rolls back like any Bills write.
pub fn new_month(
    ports: &DesktopPorts,
    ledger: &Entity<LedgerStore>,
    today: chrono::NaiveDate,
    cx: &mut App,
) {
    PopulateSchedule { today }.run(ports.borrow_mut().split_mut().bills);
    refresh(ports, ledger, cx);
}

/// Copies the Units Bills reads from `ledger` into `ports`, before an event or for a view model
/// drawn outside one.
pub fn copy_reads(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_reads(ledger);
}

/// Brings the Ledger's Bills read copy in line with the ports, announcing `Domain::Bills` when it
/// changed, and copies the reads back for the next event. Run after every Bills event and Dialog
/// confirm, so the Dashboard, the rail badge, Budgets and Documents see a write at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_bills(cx));
    copy_reads(ports, ledger.read(cx).ledger());
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_bills::BillScheduleEntry;
    use lib_database::AccountsStore as _;

    use super::*;

    fn stub() -> (Ledger, DesktopPorts) {
        let ledger = Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"));
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            payees(&ledger),
            crate::accounts::LedgerBackedPorts::in_memory(&ledger),
            categories(&ledger),
            transactions(&ledger),
        );
        (ledger, ports)
    }

    fn categories(ledger: &Ledger) -> SharedCategoriesStore {
        crate::categories::ports::in_memory(ledger)
    }

    fn transactions(ledger: &Ledger) -> SharedTransactionsStore {
        Rc::new(RefCell::new(lib_transactions::fake::InMemoryTransactions {
            transactions: ledger.transactions().to_vec(),
        }))
    }

    fn payees(ledger: &Ledger) -> SharedPayeesStore {
        crate::payees::LedgerBackedPorts::in_memory(ledger, categories(ledger))
    }

    fn entries(ports: &DesktopPorts) -> Vec<BillScheduleEntry> {
        ports.borrow().bills().entries()
    }

    #[test]
    fn bill_plan_and_entry_changes_reach_the_store() {
        let (_ledger, ports) = stub();
        let mut plan = ports.borrow().bills().bill_plans()[0].clone();
        plan.name = "Renamed".into();
        let entry = BillScheduleEntry {
            superseded: true,
            ..entries(&ports)[0].clone()
        };

        assert!(forward(
            &ports,
            &[
                LedgerChange::UpsertBillPlan(plan.clone()),
                LedgerChange::UpsertBillEntry(entry.clone()),
            ]
        ));

        assert_eq!(ports.borrow().bills().bill_plan(plan.id), Some(plan));
        assert_eq!(ports.borrow().bills().entry(entry.id()), Some(entry));
    }

    #[test]
    fn the_payees_are_read_from_the_payees_store_not_copied_from_the_ledger() {
        let (mut ledger, _) = stub();
        let store = payees(&ledger);
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            store.clone(),
            crate::accounts::LedgerBackedPorts::in_memory(&ledger),
            categories(&ledger),
            transactions(&ledger),
        );
        let mut renamed = ledger.payees()[0].clone();
        renamed.name = "Renamed in the store".into();
        lib_payees::PayeeRepository::upsert_payee(store.borrow_mut().payees_mut(), renamed.clone());
        ledger.refresh_payees(Vec::new());
        copy_reads(&ports, &ledger);

        let read = ports.borrow().payees().bill_payees();
        assert_eq!(read.len(), store.borrow().payees().payees().len());
        assert!(read.iter().any(|payee| payee.name == renamed.name));
    }

    #[test]
    fn the_accounts_are_read_from_the_accounts_store_not_copied_from_the_ledger() {
        let (mut ledger, _) = stub();
        let store = crate::accounts::LedgerBackedPorts::in_memory(&ledger);
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            payees(&ledger),
            store.clone(),
            categories(&ledger),
            transactions(&ledger),
        );
        let mut renamed = ledger.accounts()[1].clone();
        renamed.name = "Renamed in the store".into();
        store
            .borrow_mut()
            .accounts_mut()
            .upsert_account(renamed.clone());
        ledger.refresh_accounts(Vec::new());
        copy_reads(&ports, &ledger);

        let read = ports.borrow().accounts().bill_accounts();
        assert_eq!(read.len(), store.borrow().accounts().accounts().len());
        assert!(read.iter().any(|account| account.name == renamed.name));
    }

    #[test]
    fn the_categories_are_read_from_the_categories_store_not_copied_from_the_ledger() {
        let (mut ledger, _) = stub();
        let store = categories(&ledger);
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            payees(&ledger),
            crate::accounts::LedgerBackedPorts::in_memory(&ledger),
            store.clone(),
            transactions(&ledger),
        );
        let mut renamed = ledger.categories()[6].clone();
        renamed.name = "Renamed in the store".into();
        store
            .borrow_mut()
            .categories_mut()
            .upsert_category(renamed.clone());
        ledger.refresh_categories(Vec::new());
        copy_reads(&ports, &ledger);

        let read = ports.borrow().categories().bill_categories();
        assert_eq!(read.len(), store.borrow().categories().categories().len());
        assert!(read.iter().any(|category| category.name == renamed.name));
    }

    #[test]
    fn the_splits_are_read_from_the_transactions_store_not_copied_from_the_ledger() {
        let (ledger, _) = stub();
        let store = transactions(&ledger);
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            payees(&ledger),
            crate::accounts::LedgerBackedPorts::in_memory(&ledger),
            categories(&ledger),
            store.clone(),
        );
        let mut booked = ledger.transactions()[0].clone();
        booked.id = lib_core::TransactionId::new();
        booked.description = Some("Booked in the store".into());
        store.borrow_mut().upsert_transaction(booked.clone());
        copy_reads(&ports, &ledger);

        let read = ports
            .borrow()
            .transactions()
            .splits_between(booked.date, booked.date);
        assert!(
            read.iter()
                .any(|split| split.description == booked.description)
        );
        let paid = ports.borrow().transactions().paid_split(SplitRef {
            transaction_id: booked.id,
            split_index: 0,
        });
        assert_eq!(
            paid.map(|split| split.description),
            Some(booked.description)
        );
    }

    #[test]
    fn the_reads_are_the_ledgers() {
        let (ledger, ports) = stub();
        let ports = ports.borrow();
        let ids: Vec<_> = ports
            .accounts()
            .bill_accounts()
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(
            ids,
            ledger.accounts().iter().map(|a| a.id).collect::<Vec<_>>()
        );
        let base = ledger.base_unit_row().map(|unit| unit.id);
        assert_eq!(ports.units().base_unit(), base);
        assert_eq!(
            base.and_then(|id| ports.units().unit_code(id)).as_deref(),
            Some("aud")
        );
    }
}
