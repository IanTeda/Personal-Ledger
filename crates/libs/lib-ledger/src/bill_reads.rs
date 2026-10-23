//! Halfway shims (#774) between `lib-bills` and the domains that still live on the Ledger:
//!
//! - **`BillAccounts`, `BillCategories` and `BillPayees` for `Ledger`.** `lib-bills`' read ports
//!   answered from the stub's Accounts, Categories and Payees, which aren't persisted, mapped onto
//!   Bills' own read types. Each flag is filled by its owning domain's rule, so the rule lives once
//!   (`docs/development/viper-architecture.md`, Rules > Dependencies). Each mapping moves to
//!   `lib-database` as its domain migrates. The Transactions read port is `lib-database`'s;
//!   [`Ledger::bill_splits`] flattens the Ledger's copy the same way, for a Client that copies
//!   Bills' reads from the Ledger.
//! - **The Bills read copy ([`Ledger::refresh_bills`]).** The Bills ports are the truth; the
//!   Ledger keeps a copy for the Dashboard, the rail badge, Budgets and Documents. It goes when
//!   the last of them reads Bills through `lib-bills`.

use lib_accounts::Account;
use lib_bills::{
    BillAccount, BillAccounts, BillCategories, BillCategory, BillPayee, BillPayees, BillPlan,
    BillScheduleEntry, BillSplit, SplitRef,
};
use lib_categories::Category;
use lib_core::CategoryTypes;
use lib_payees::Payee;
use lib_transactions::Transaction;

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

    /// The Ledger's Transactions as Bills reads them, a [`BillSplit`] per Split, in the
    /// Transactions' order.
    pub fn bill_splits(&self) -> Vec<BillSplit> {
        bill_splits(&self.transactions)
    }
}

/// `transactions` flattened as Bills reads them: each Split with its Transaction's date, Account
/// and memo.
pub(crate) fn bill_splits(transactions: &[Transaction]) -> Vec<BillSplit> {
    transactions
        .iter()
        .flat_map(|transaction| {
            transaction
                .splits
                .iter()
                .enumerate()
                .map(|(split_index, split)| BillSplit {
                    split: SplitRef {
                        transaction_id: transaction.id,
                        split_index,
                    },
                    date: transaction.date,
                    account_id: transaction.account_id,
                    description: transaction.description.clone(),
                    amount: split.amount.clone(),
                    category_id: split.category_id,
                    payee_id: split.payee_id,
                })
        })
        .collect()
}

/// `accounts` as Bills reads them.
pub(crate) fn bill_accounts(accounts: &[Account]) -> Vec<BillAccount> {
    accounts
        .iter()
        .map(|account| BillAccount {
            id: account.id,
            name: account.name.clone(),
            unit: account.unit,
            takes_transactions: lib_transactions::takes_transactions(&account.account_type),
        })
        .collect()
}

/// `categories` as Bills reads them.
pub(crate) fn bill_categories(categories: &[Category]) -> Vec<BillCategory> {
    categories
        .iter()
        .map(|category| BillCategory {
            id: category.id,
            name: category.name.clone(),
            path: lib_categories::path(categories, category.id)
                .unwrap_or_else(|| category.name.clone()),
            is_expense: category.category_type == CategoryTypes::Expense,
            is_leaf: lib_categories::is_leaf(categories, category.id),
        })
        .collect()
}

/// `payees` as Bills reads them.
pub(crate) fn bill_payees(payees: &[Payee]) -> Vec<BillPayee> {
    payees
        .iter()
        .map(|payee| BillPayee {
            id: payee.id,
            name: payee.name.clone(),
            is_active: payee.is_active,
        })
        .collect()
}

impl BillAccounts for Ledger {
    fn bill_accounts(&self) -> Vec<BillAccount> {
        bill_accounts(&self.accounts)
    }
}

impl BillCategories for Ledger {
    fn bill_categories(&self) -> Vec<BillCategory> {
        bill_categories(&self.categories)
    }
}

impl BillPayees for Ledger {
    fn bill_payees(&self) -> Vec<BillPayee> {
        bill_payees(&self.payees)
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
    fn the_read_ports_answer_every_row_flagged_by_its_domains_rules() {
        let ledger = Ledger::stub_on(today());
        let ids = |accounts: Vec<BillAccount>| accounts.iter().map(|a| a.id).collect::<Vec<_>>();
        assert_eq!(
            ids(ledger.bill_accounts()),
            ledger.accounts().iter().map(|a| a.id).collect::<Vec<_>>()
        );
        assert_eq!(ledger.bill_categories().len(), ledger.categories().len());
        assert_eq!(ledger.bill_payees().len(), ledger.payees().len());
        let splits: usize = ledger.transactions().iter().map(|t| t.splits.len()).sum();
        assert_eq!(ledger.bill_splits().len(), splits);
        let split = ledger
            .transactions()
            .iter()
            .find(|t| t.is_split())
            .expect("the stub has a split Transaction");
        let second = ledger
            .bill_splits()
            .into_iter()
            .find(|s| s.split.transaction_id == split.id && s.split.split_index == 1)
            .expect("its second Split is read");
        assert_eq!(
            (second.date, second.account_id, &second.description),
            (split.date, split.account_id, &split.description)
        );
        assert_eq!(second.amount, split.splits[1].amount);
        assert_eq!(second.category_id, split.splits[1].category_id);
        assert_eq!(second.payee_id, split.splits[1].payee_id);

        let home_loan = ledger
            .bill_accounts()
            .into_iter()
            .find(|a| a.name == "Home Loan")
            .expect("the stub has a Home Loan");
        assert!(!home_loan.takes_transactions);
        let water = ledger
            .bill_categories()
            .into_iter()
            .find(|c| c.name == "Water")
            .expect("the stub has Water");
        assert_eq!(water.path, "Housing \u{203a} Utilities \u{203a} Water");
        assert!(water.is_expense && water.is_leaf);
        let utilities = ledger
            .bill_categories()
            .into_iter()
            .find(|c| c.name == "Utilities")
            .expect("the stub has Utilities");
        assert!(!utilities.is_leaf);
        assert!(ledger.bill_payees().iter().any(|p| !p.is_active));
    }

    /// `lib-bills`' fake stub stands in for these mappings and for `PayBill`, so its tests run on
    /// what the Ledger's stub holds.
    #[test]
    fn the_bills_fake_stub_is_the_ledgers() {
        let ledger = Ledger::stub_on(today());
        assert_eq!(ledger.bill_accounts(), lib_bills::fake::stub_accounts());
        assert_eq!(ledger.bill_categories(), lib_bills::fake::stub_categories());
        let payees = ledger.bill_payees();
        for payee in lib_bills::fake::stub_payees() {
            assert!(payees.contains(&payee), "{payee:?}");
        }

        let fake = lib_bills::fake::InMemoryPorts::stub(today());
        let mut transactions = Vec::new();
        let booked = crate::bills_seed::book_stub_bills(
            lib_bills::default_bills(
                &ledger.bill_accounts(),
                &ledger.bill_categories(),
                &payees,
                today(),
            ),
            &mut transactions,
        );
        assert_eq!(fake.bills.plans, booked.plans);
        assert_eq!(fake.bills.entries, booked.entries);
        assert_eq!(fake.reads.splits, bill_splits(&transactions));
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
