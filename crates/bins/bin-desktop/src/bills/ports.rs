//! The Bills ports as the Desktop holds them: the store `shell::router::build` is given, shared by
//! the Bills module and `LedgerStore`, the Payees store Bills reads its Payees from (#831), the
//! Accounts store it reads its Accounts from (#853), the Categories store it reads its
//! Categories from (#858), the Transactions store it reads its Splits from (#926) and the Units
//! store it reads its Unit codes from (#993), and the announcement of a Bills write (#1028). The
//! write is committed by the input it was made in (`crate::input::send`), not here.
//!
//! The store is shared with the Bill Payments module too, whose Pay it directly writes an entry,
//! the Account Deletion module, whose delete Unmatches the entries an Account's Transactions paid
//! (#854), and the Category Deletion module, whose delete moves the Bill Plans naming the Category
//! to Uncategorised (#1005).

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_bills::interactor::PopulateSchedule;
use lib_bills::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillTransactions, BillUnits, BillsPorts, BillsRepository, PortsMut,
};
use lib_database::FakeStores;
use lib_database::{
    SharedAccountsStore, SharedBillsStore, SharedCategoriesStore, SharedPayeesStore,
    SharedTransactionsStore, SharedUnitsStore, StoredBillReads, StoredBillUnits,
};

use crate::ledger_store::LedgerStore;

/// The Bills ports the Bills module and `LedgerStore` share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The store's two repositories as the one a Bills Interactor writes through. It forwards rather
/// than lending the store's own: `bills()` returns a borrow that must outlive the call, and the
/// store is shared behind a `RefCell` whose guard can't be lent out of it. It notes each write, so
/// the Bills are announced only after an event that wrote them ([`announce_written`]).
struct Store {
    rows: SharedBillsStore,
    wrote: bool,
}

impl BillPlanRepository for Store {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.rows.borrow().plans().bill_plans()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        self.wrote = true;
        self.rows.borrow_mut().plans_mut().upsert_bill_plan(plan);
    }
}

impl BillScheduleRepository for Store {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.rows.borrow().entries().entries()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        self.wrote = true;
        self.rows.borrow_mut().entries_mut().upsert_entry(entry);
    }
}

/// `store`'s Bill Plans and Bill Schedule, `reads`' Payees, Accounts, Categories and Splits and
/// `units`' Unit codes.
pub struct LedgerBackedPorts {
    store: Store,
    reads: StoredBillReads,
    units: StoredBillUnits,
}

impl LedgerBackedPorts {
    pub fn shared(
        store: SharedBillsStore,
        payees: SharedPayeesStore,
        accounts: SharedAccountsStore,
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
        units: SharedUnitsStore,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store: Store {
                rows: store,
                wrote: false,
            },
            reads: StoredBillReads {
                payees,
                accounts,
                categories,
                transactions,
            },
            units: StoredBillUnits { units },
        }))
    }

    /// `stub`'s Bill Plans and Bill Schedule in memory, reading `payees`, `accounts`,
    /// `categories`, `transactions` and `units`, for a test that needs no storage.
    pub fn in_memory(
        stub: &FakeStores,
        payees: SharedPayeesStore,
        accounts: SharedAccountsStore,
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
        units: SharedUnitsStore,
    ) -> DesktopPorts {
        Self::shared(
            in_memory_store(stub),
            payees,
            accounts,
            categories,
            transactions,
            units,
        )
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    pub fn reload(&mut self) -> lib_database::Result<()> {
        self.store.rows.borrow_mut().reload()
    }

    /// The Bills store the ports write, which the Bill Payments module writes too.
    pub fn store(&self) -> SharedBillsStore {
        self.store.rows.clone()
    }

    /// Whether a Bills Interactor wrote through these ports since the last ask.
    fn take_wrote(&mut self) -> bool {
        std::mem::take(&mut self.store.wrote)
    }
}

impl BillsPorts for LedgerBackedPorts {
    fn bills(&self) -> &dyn BillsRepository {
        &self.store
    }

    fn transactions(&self) -> &dyn BillTransactions {
        &self.reads
    }

    fn accounts(&self) -> &dyn BillAccounts {
        &self.reads
    }

    fn categories(&self) -> &dyn BillCategories {
        &self.reads
    }

    fn payees(&self) -> &dyn BillPayees {
        &self.reads
    }

    fn units(&self) -> &dyn BillUnits {
        &self.units
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            bills: &mut self.store,
            transactions: &self.reads,
            accounts: &self.reads,
            categories: &self.reads,
        }
    }
}

/// The stub Units, base Unit and Price Sources as of `now`, as an in-memory Units store, for a test
/// that needs no storage: the store the Units ports share and the Bills, Tags, Accounts and
/// Institutions read Units from.
pub fn in_memory_units(now: chrono::DateTime<chrono::Utc>) -> SharedUnitsStore {
    Rc::new(RefCell::new(lib_units::fake::InMemoryPorts::stub(now)))
}

/// `stub`'s Bill Plans and Bill Schedule as an in-memory store, for a test that needs no storage
/// and builds the readers of the store before its ports.
pub fn in_memory_store(stub: &FakeStores) -> SharedBillsStore {
    Rc::new(RefCell::new(stub.bills.clone()))
}

/// Brings the stored Bill Schedule up to the horizon as of `today` (ADR-0019's "on Client start"),
/// writing the entries generation adds. The caller commits them.
pub fn generate(store: &mut dyn BillsRepository, today: chrono::NaiveDate) {
    PopulateSchedule { today }.run(store);
}

/// Brings the Bill Schedule up to the horizon for the month a running Desktop has crossed into,
/// wherever it is (ADR-0019), then announces the Bills as after a Bills event, so the
/// Dashboard and Budgets redraw on the new entries. The day poll's input commits them as
/// one write, and a failed save rolls back like any Bills write.
pub fn new_month(
    ports: &DesktopPorts,
    ledger: &Entity<LedgerStore>,
    today: chrono::NaiveDate,
    cx: &mut App,
) {
    PopulateSchedule { today }.run(ports.borrow_mut().split_mut().bills);
    announce_written(ports, ledger, cx);
}

/// Announces `Domain::Bills`, so the Dashboard, Bills, Budgets and Documents redraw on a write at
/// once (#1028): for a module that wrote the Bills store directly, such as Bill Payments.
pub fn announce(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.announce_bills(cx));
}

/// Announces `Domain::Bills` after a Bills event or Dialog input, but only when it wrote through
/// `ports`, so a cursor move or a keystroke into a field redraws no other View.
pub fn announce_written(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    if ports.borrow_mut().take_wrote() {
        announce(ledger, cx);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_bills::{BillScheduleEntry, SplitRef};
    use lib_database::AccountsStore as _;

    use super::*;

    fn stub() -> (FakeStores, DesktopPorts) {
        let fakes = FakeStores::stub(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"));
        let ports = LedgerBackedPorts::in_memory(
            &fakes,
            payees(&fakes),
            crate::accounts::LedgerBackedPorts::in_memory(crate::accounts::ports::in_memory_reads(
                &fakes,
            )),
            categories(),
            transactions(&fakes),
            in_memory_units(chrono::Utc::now()),
        );
        (fakes, ports)
    }

    fn categories() -> SharedCategoriesStore {
        crate::categories::ports::in_memory()
    }

    fn transactions(fakes: &FakeStores) -> SharedTransactionsStore {
        Rc::new(RefCell::new(fakes.transactions.clone()))
    }

    fn payees(fakes: &FakeStores) -> SharedPayeesStore {
        crate::payees::LedgerBackedPorts::in_memory(lib_database::StoredPayeeReads {
            transactions: transactions(fakes),
            bills: in_memory_store(fakes),
            categories: categories(),
        })
    }

    fn entries(ports: &DesktopPorts) -> Vec<BillScheduleEntry> {
        ports.borrow().bills().entries()
    }

    #[test]
    fn the_payees_are_read_from_the_payees_store_not_copied_from_the_ledger() {
        let (fakes, _) = stub();
        let store = payees(&fakes);
        let ports = LedgerBackedPorts::in_memory(
            &fakes,
            store.clone(),
            crate::accounts::LedgerBackedPorts::in_memory(crate::accounts::ports::in_memory_reads(
                &fakes,
            )),
            categories(),
            transactions(&fakes),
            in_memory_units(chrono::Utc::now()),
        );
        let mut renamed = store.borrow().payees().payees()[0].clone();
        renamed.name = "Renamed in the store".into();
        lib_payees::PayeeRepository::upsert_payee(store.borrow_mut().payees_mut(), renamed.clone());

        let read = ports.borrow().payees().bill_payees();
        assert_eq!(read.len(), store.borrow().payees().payees().len());
        assert!(read.iter().any(|payee| payee.name == renamed.name));
    }

    #[test]
    fn the_accounts_are_read_from_the_accounts_store_not_copied_from_the_ledger() {
        let (fakes, _) = stub();
        let store = crate::accounts::LedgerBackedPorts::in_memory(
            crate::accounts::ports::in_memory_reads(&fakes),
        );
        let ports = LedgerBackedPorts::in_memory(
            &fakes,
            payees(&fakes),
            store.clone(),
            categories(),
            transactions(&fakes),
            in_memory_units(chrono::Utc::now()),
        );
        let mut renamed = lib_accounts::default_accounts()[1].clone();
        renamed.name = "Renamed in the store".into();
        store
            .borrow_mut()
            .accounts_mut()
            .upsert_account(renamed.clone());

        let read = ports.borrow().accounts().bill_accounts();
        assert_eq!(read.len(), store.borrow().accounts().accounts().len());
        assert!(read.iter().any(|account| account.name == renamed.name));
    }

    #[test]
    fn the_categories_are_read_from_the_categories_store_not_copied_from_the_ledger() {
        let (fakes, _) = stub();
        let store = categories();
        let ports = LedgerBackedPorts::in_memory(
            &fakes,
            payees(&fakes),
            crate::accounts::LedgerBackedPorts::in_memory(crate::accounts::ports::in_memory_reads(
                &fakes,
            )),
            store.clone(),
            transactions(&fakes),
            in_memory_units(chrono::Utc::now()),
        );
        let mut renamed = store.borrow().categories().categories()[6].clone();
        renamed.name = "Renamed in the store".into();
        store
            .borrow_mut()
            .categories_mut()
            .upsert_category(renamed.clone());

        let read = ports.borrow().categories().bill_categories();
        assert_eq!(read.len(), store.borrow().categories().categories().len());
        assert!(read.iter().any(|category| category.name == renamed.name));
    }

    #[test]
    fn the_splits_are_read_from_the_transactions_store_not_copied_from_the_ledger() {
        let (fakes, _) = stub();
        let store = transactions(&fakes);
        let ports = LedgerBackedPorts::in_memory(
            &fakes,
            payees(&fakes),
            crate::accounts::LedgerBackedPorts::in_memory(crate::accounts::ports::in_memory_reads(
                &fakes,
            )),
            categories(),
            store.clone(),
            in_memory_units(chrono::Utc::now()),
        );
        let mut booked = fakes.transactions.transactions[0].clone();
        booked.id = lib_core::TransactionId::new();
        booked.description = Some("Booked in the store".into());
        store.borrow_mut().upsert_transaction(booked.clone());

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
        let (_fakes, ports) = stub();
        let ports = ports.borrow();
        let ids: Vec<_> = ports
            .accounts()
            .bill_accounts()
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(
            ids,
            lib_accounts::default_accounts()
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>()
        );
        let base = ports.units().base_unit();
        assert_eq!(base, Some(lib_core::UnitId::SEED_AUD));
        assert_eq!(
            base.and_then(|id| ports.units().unit_code(id)).as_deref(),
            Some("aud")
        );
    }
}
