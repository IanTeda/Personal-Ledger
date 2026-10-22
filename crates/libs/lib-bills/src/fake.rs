//! The in-memory Bills ports, for tests and UI-first work. Each holds its rows in plain fields so a
//! test can set up or read state directly. The stub seed is `seed`'s [`default_bills`](crate::default_bills).

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::Category;
use lib_core::TransactionId;
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::{
    BillAccounts, BillCategories, BillPayees, BillPlan, BillPlanRepository, BillScheduleEntry,
    BillScheduleRepository, BillTransactions, BillsSeed,
};

/// The Bill Plans and the Bill Schedule, stored in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBills {
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

impl From<BillsSeed> for InMemoryBills {
    fn from(seed: BillsSeed) -> Self {
        Self {
            plans: seed.plans,
            entries: seed.entries,
        }
    }
}

impl BillPlanRepository for InMemoryBills {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.plans.clone()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        match self.plans.iter_mut().find(|row| row.id == plan.id) {
            Some(row) => *row = plan,
            None => self.plans.push(plan),
        }
    }
}

impl BillScheduleRepository for InMemoryBills {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.entries.clone()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        match self.entries.iter_mut().find(|row| row.id() == entry.id()) {
            Some(row) => {
                *row = BillScheduleEntry {
                    row_id: row.row_id,
                    ..entry
                }
            }
            None => self.entries.push(entry),
        }
    }
}

/// The other domains' rows Bills reads, held in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryBillReads {
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub payees: Vec<Payee>,
    pub transactions: Vec<Transaction>,
}

impl BillTransactions for InMemoryBillReads {
    fn transactions_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<Transaction> {
        crate::transactions_between(&self.transactions, from, to)
    }

    fn paid_transaction(&self, id: TransactionId) -> Option<Transaction> {
        self.transactions.iter().find(|t| t.id == id).cloned()
    }
}

impl BillAccounts for InMemoryBillReads {
    fn bill_accounts(&self) -> Vec<Account> {
        self.accounts.clone()
    }
}

impl BillCategories for InMemoryBillReads {
    fn bill_categories(&self) -> Vec<Category> {
        self.categories.clone()
    }
}

impl BillPayees for InMemoryBillReads {
    fn bill_payees(&self) -> Vec<Payee> {
        self.payees.clone()
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, BillPlanId, BillScheduleEntryId, TransactionStatus};

    use super::*;
    use crate::Resolution;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).expect("a September day")
    }

    #[test]
    fn an_upserted_entry_keeps_the_stored_id_for_its_plan_and_due_date() {
        let stored = BillScheduleEntry {
            row_id: BillScheduleEntryId::seed(1),
            plan_id: BillPlanId::seed(1),
            due: date(1),
            resolution: Resolution::Unresolved,
            superseded: false,
        };
        let mut bills = InMemoryBills {
            plans: Vec::new(),
            entries: vec![stored.clone()],
        };

        bills.upsert_entry(BillScheduleEntry {
            row_id: BillScheduleEntryId::seed(2),
            superseded: true,
            ..stored.clone()
        });

        assert_eq!(
            bills.entries,
            [BillScheduleEntry {
                superseded: true,
                ..stored
            }]
        );
    }

    #[test]
    fn the_window_includes_both_ends() {
        let on = |day| Transaction {
            id: TransactionId::seed(day),
            date: date(day),
            account_id: AccountId::seed(1),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: Vec::new(),
        };
        let reads = InMemoryBillReads {
            transactions: (1..=5).map(on).collect(),
            ..InMemoryBillReads::default()
        };

        let mut days: Vec<u32> = reads
            .transactions_between(date(2), date(4))
            .iter()
            .map(|t| t.id.seed_number())
            .collect();
        days.sort_unstable();

        assert_eq!(days, [2, 3, 4]);
        assert_eq!(reads.paid_transaction(TransactionId::seed(5)), Some(on(5)));
    }
}
