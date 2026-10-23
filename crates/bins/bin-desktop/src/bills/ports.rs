//! The Bills ports as the Desktop holds them: the store `build_shell` is given, shared by the
//! Bills module and `LedgerStore`, with the Accounts, Categories, Payees, Splits and Units
//! Bills reads copied from the Ledger (#778's halfway shims), and the step that commits a Bills
//! write and keeps the Ledger's read copy in line after it.
//!
//! The reads are copied before each Bills event, since a port can't reach the `LedgerStore`
//! Entity without a gpui context; each goes when its domain answers through its own port.
//! `LedgerStore` writes the store too, through [`forward`]: the Bills writers not yet migrated
//! (Category reassign and Account delete's Unmatch) still run as `LedgerCommand`s. The store is
//! shared with the Bill Payments module too, whose Pay it directly writes an entry.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_bills::fake::{InMemoryBillReads, InMemoryBillUnits};
use lib_bills::interactor::PopulateSchedule;
use lib_bills::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillTransactions, BillUnits, BillsPorts, BillsRepository, PortsMut,
    unmatch_transaction,
};
use lib_database::{Commit, SharedBillsStore};
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

/// `store`'s Bill Plans and Bill Schedule, with what Bills reads of the other domains copied from
/// the Ledger, and the commit that ends each Interactor call.
pub struct LedgerBackedPorts {
    store: Store,
    commit: Rc<dyn Commit>,
    reads: InMemoryBillReads,
    units: InMemoryBillUnits,
}

impl LedgerBackedPorts {
    pub fn shared(store: SharedBillsStore, commit: Rc<dyn Commit>) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store: Store(store),
            commit,
            reads: InMemoryBillReads::default(),
            units: InMemoryBillUnits::default(),
        }))
    }

    /// `ledger`'s own Bill Plans and Bill Schedule in memory, for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger) -> DesktopPorts {
        let ports = Self::shared(
            Rc::new(RefCell::new(lib_bills::fake::InMemoryBills {
                plans: ledger.bill_plans().to_vec(),
                entries: ledger.bill_entries().to_vec(),
            })),
            Rc::new(lib_database::InMemoryCommit),
        );
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

    /// Copies what Bills reads of the other domains from `ledger`, so the next Interactor and view
    /// model read it.
    fn copy_reads(&mut self, ledger: &Ledger) {
        self.reads = InMemoryBillReads {
            accounts: ledger.bill_accounts(),
            categories: ledger.bill_categories(),
            payees: ledger.bill_payees(),
            splits: ledger.bill_splits(),
        };
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

/// Sends each Bill Plan and entry in `changes` to the store, in the order recorded, and Unmatches
/// the entries a deleted Transaction paid, since deleting it deletes the Splits their Matches are
/// on (ADR-0019). The writers it carries: Category reassign, and Account delete's Unmatch.
/// Returns whether it wrote anything, so the caller refreshes the read copy.
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
            LedgerChange::DeleteTransaction(id) => {
                let before = store.entries();
                let mut entries = before.clone();
                unmatch_transaction(&mut entries, *id);
                for entry in entries.into_iter().filter(|entry| !before.contains(entry)) {
                    store.upsert_entry(entry);
                    wrote = true;
                }
            }
            _ => {}
        }
    }
    wrote
}

/// Brings the stored Bill Schedule up to the horizon as of `today` (ADR-0019's "on Client start"),
/// writing the entries generation adds. The caller commits them.
pub fn generate(store: &mut dyn BillsRepository, today: chrono::NaiveDate) {
    let written = PopulateSchedule { today }.run(store);
    tracing::debug!(written, "Generated the Bill Schedule on start");
}

/// Brings the Bill Schedule up to the horizon for the month a running Desktop has crossed into,
/// wherever it is (ADR-0019), then commits and refreshes the read copy as after a Bills event, so
/// the Dashboard, the rail badge and Budgets see the new entries. A failed save rolls back like
/// any Bills write.
pub fn new_month(
    ports: &DesktopPorts,
    ledger: &Entity<LedgerStore>,
    today: chrono::NaiveDate,
    cx: &mut App,
) {
    let written = PopulateSchedule { today }.run(ports.borrow_mut().split_mut().bills);
    tracing::info!(written, %today, "Generated the Bill Schedule for a new month");
    sync(ports, ledger, cx);
}

/// Copies what Bills reads of the other domains from `ledger` into `ports`, before an event or
/// for a view model drawn outside one.
pub fn copy_reads(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_reads(ledger);
}

/// Commits the Interactor call just made, if any, then brings the Ledger's Bills read copy in line
/// with the ports, announcing `Domain::Bills` when it changed, and copies the reads back for the
/// next event. Run after every Bills event and Dialog confirm, so a write is saved as one
/// transaction and the Dashboard, the rail badge, Budgets and Documents see it at once.
pub fn sync(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ports.borrow().commit.commit();
    ledger.update(cx, |store, cx| store.refresh_bills(cx));
    copy_reads(ports, ledger.read(cx).ledger());
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_bills::{BillScheduleEntry, Resolution};

    use super::*;

    fn stub() -> (Ledger, DesktopPorts) {
        let ledger = Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"));
        let ports = LedgerBackedPorts::in_memory(&ledger);
        (ledger, ports)
    }

    fn entries(ports: &DesktopPorts) -> Vec<BillScheduleEntry> {
        ports.borrow().bills().entries()
    }

    #[test]
    fn a_deleted_transaction_unmatches_the_entries_it_paid() {
        let (_ledger, ports) = stub();
        let (entry_id, transaction_id) = entries(&ports)
            .iter()
            .find_map(|entry| match entry.resolution {
                Resolution::Paid(split) => Some((entry.id(), split.transaction_id)),
                _ => None,
            })
            .expect("the stub pays an entry");

        assert!(forward(
            &ports,
            &[LedgerChange::DeleteTransaction(transaction_id)]
        ));

        assert_eq!(
            lib_bills::entry(&entries(&ports), entry_id).map(|entry| entry.resolution.clone()),
            Some(Resolution::Unresolved)
        );
    }

    #[test]
    fn a_deleted_transaction_that_paid_nothing_writes_nothing() {
        let (ledger, ports) = stub();
        let before = entries(&ports);
        let unpaid = ledger
            .transactions()
            .iter()
            .find(|t| !lib_bills::carries_bill(&before, t.id))
            .expect("the stub has a Transaction no entry names");

        assert!(!forward(
            &ports,
            &[LedgerChange::DeleteTransaction(unpaid.id)]
        ));
        assert_eq!(entries(&ports), before);
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
    fn the_reads_are_the_ledgers() {
        let (ledger, ports) = stub();
        let ports = ports.borrow();
        assert_eq!(ports.accounts().bill_accounts(), ledger.bill_accounts());
        let base = ledger.base_unit_row().map(|unit| unit.id);
        assert_eq!(ports.units().base_unit(), base);
        assert_eq!(
            base.and_then(|id| ports.units().unit_code(id)).as_deref(),
            Some("aud")
        );
    }
}
