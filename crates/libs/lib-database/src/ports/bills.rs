//! `lib-bills`' ports answered over the stores: the Bill Plans and Bill Schedule from the Bills
//! store, the Payees, Accounts, Categories and Splits a Bill reads through [`StoredBillReads`]
//! (#831, #853, #858, #926), and its Unit codes through [`StoredBillUnits`] (#993, #1084).

use lib_bills::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillTransactions, BillUnits, BillsPorts, BillsRepository, PortsMut,
};

use crate::adapter::store::{SharedBillsStore, StoredBillReads, StoredBillUnits};

/// The shared Bills store's two repositories as the one a Bills Interactor writes through,
/// borrowed for each call: the store's guard can't be lent out of its `RefCell`.
struct Bills(SharedBillsStore);

impl BillPlanRepository for Bills {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.0.borrow().plans().bill_plans()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        self.0.borrow_mut().plans_mut().upsert_bill_plan(plan);
    }
}

impl BillScheduleRepository for Bills {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.0.borrow().entries().entries()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        self.0.borrow_mut().entries_mut().upsert_entry(entry);
    }
}

/// [`lib_bills::BillsPorts`] over the shared Bills store, and the reads over the Payees, Accounts,
/// Categories, Transactions and Units stores. A write reaches the store's recorder, so the commit
/// names the Bills in what the input wrote.
pub struct StoredBillsPorts {
    bills: Bills,
    reads: StoredBillReads,
    units: StoredBillUnits,
}

impl StoredBillsPorts {
    pub fn new(bills: SharedBillsStore, reads: StoredBillReads, units: StoredBillUnits) -> Self {
        Self {
            bills: Bills(bills),
            reads,
            units,
        }
    }
}

impl BillsPorts for StoredBillsPorts {
    fn bills(&self) -> &dyn BillsRepository {
        &self.bills
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
            bills: &mut self.bills,
            transactions: &self.reads,
            accounts: &self.reads,
            categories: &self.reads,
        }
    }
}

#[cfg(test)]
mod tests {
    use lib_bills::interactor::SkipBill;

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn each_bills_write_reports_the_bills_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts { bills, commit, .. } = ledger;
            let mut plan = bills
                .borrow()
                .bills()
                .bill_plans()
                .into_iter()
                .next()
                .expect("a sample Bill Plan");
            plan.name.push_str(" renamed");

            bills.borrow_mut().split_mut().bills.upsert_bill_plan(plan);
            assert_eq!(commit.commit().wrote, [Domain::Bills], "a Plan");

            let open = bills
                .borrow()
                .bills()
                .entries()
                .into_iter()
                .find(lib_bills::BillScheduleEntry::is_open)
                .expect("an open entry")
                .id();
            SkipBill { entry: open }
                .run(bills.borrow_mut().split_mut().bills)
                .expect("an open entry");
            assert_eq!(commit.commit().wrote, [Domain::Bills], "a skip");
        }
    }
}
