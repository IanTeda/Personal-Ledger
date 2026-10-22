//! Halfway shims (#774) between `lib-bills` and the domains that still live on the Ledger:
//!
//! - **`BillAccounts`, `BillCategories` and `BillPayees` for `Ledger`.** `lib-bills`' read ports
//!   answered from the stub's Accounts, Categories and Payees, which aren't persisted. Each moves
//!   to `lib-database` as its domain migrates. The Transactions read port is `lib-database`'s.
//! - **The Bills read copy ([`Ledger::refresh_bills`]).** The Bills ports are the truth; the
//!   Ledger keeps a copy for today's Bills View, the Dashboard, Budgets and Documents. It goes when
//!   the last of them reads Bills through `lib-bills`.

use lib_accounts::Account;
use lib_bills::{BillAccounts, BillCategories, BillPayees, BillPlan, BillScheduleEntry};
use lib_categories::Category;
use lib_payees::Payee;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Bill Plans and the Bill Schedule with what the Bills ports
    /// hold, the entries by due date and then Plan as generation leaves them, since a port lists
    /// them in no particular order. Returns whether anything changed, so a Client announces only
    /// a real change.
    pub fn refresh_bills(
        &mut self,
        plans: Vec<BillPlan>,
        mut entries: Vec<BillScheduleEntry>,
    ) -> bool {
        entries.sort_by_key(|entry| (entry.due, entry.plan_id));
        if self.bill_plans == plans && self.bill_entries == entries {
            return false;
        }
        self.bill_plans = plans;
        self.bill_entries = entries;
        true
    }
}

impl BillAccounts for Ledger {
    fn bill_accounts(&self) -> Vec<Account> {
        self.accounts.clone()
    }
}

impl BillCategories for Ledger {
    fn bill_categories(&self) -> Vec<Category> {
        self.categories.clone()
    }
}

impl BillPayees for Ledger {
    fn bill_payees(&self) -> Vec<Payee> {
        self.payees.clone()
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    #[test]
    fn the_read_ports_answer_the_ledgers_rows() {
        let ledger = Ledger::stub_on(today());
        assert_eq!(ledger.bill_accounts(), ledger.accounts());
        assert_eq!(ledger.bill_categories(), ledger.categories());
        assert_eq!(ledger.bill_payees(), ledger.payees());
    }

    #[test]
    fn a_refresh_sorts_the_entries_and_reports_only_a_change() {
        let mut ledger = Ledger::stub_on(today());
        let plans = ledger.bill_plans().to_vec();
        let mut entries = ledger.bill_entries().to_vec();
        entries.reverse();

        assert!(!ledger.refresh_bills(plans.clone(), entries.clone()));

        entries[0].superseded = !entries[0].superseded;
        assert!(ledger.refresh_bills(plans, entries));
        assert!(
            ledger
                .bill_entries()
                .is_sorted_by_key(|entry| (entry.due, entry.plan_id))
        );
    }

    #[test]
    fn stub_entry_ids_follow_their_plan_and_due_date_whatever_the_day() {
        let a = Ledger::stub_on(today());
        let b = Ledger::stub_on(today().succ_opt().expect("a real date"));
        let shared: Vec<_> = a
            .bill_entries()
            .iter()
            .filter_map(|entry| {
                lib_bills::entry(b.bill_entries(), entry.id()).map(|other| (entry, other))
            })
            .collect();

        assert!(!shared.is_empty());
        assert!(shared.iter().all(|(x, y)| x.row_id == y.row_id));
        let mut ids: Vec<_> = a.bill_entries().iter().map(|e| e.row_id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            a.bill_entries().len(),
            "every entry has its own id"
        );
    }
}
