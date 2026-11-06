//! The Ledger aggregate: every domain collection, joined into one stub dataset.

mod account_reads;
mod account_usage;
mod bill_reads;
#[cfg(test)]
mod bills_seed_tests;
mod budget_reads;
#[cfg(test)]
mod budgets_seed_tests;
mod category_reads;
mod change;
mod document_reads;
#[cfg(test)]
mod documents_seed_tests;
pub mod seed;
mod tag_usage;

pub use budget_reads::{budget_accounts, budget_categories};
pub use change::Domain;
pub use seed::{DEFAULT_SEED, HISTORY_DAYS, default_transactions};
pub use tag_usage::{TagRows, transaction_count};

use chrono::{Local, NaiveDate};
use lib_accounts::{Account, default_accounts};
use lib_bills::{BillPlan, BillScheduleEntry};
use lib_budgets::{Budgets, default_budgets};
use lib_categories::{Category, default_categories};
use lib_institutions::{Institution, default_institutions};
use lib_inventory::{Inventory, default_inventory};
use lib_payees::{Payee, default_payees};
use lib_tags::{Tag, default_tags};
use lib_transactions::Transaction;

/// Every domain collection the Clients read, owned in one place. Nothing writes it but the read
/// copies' refreshes: every domain is written by its module, through its ports (#1006).
#[derive(Debug, Clone)]
pub struct Ledger {
    accounts: Vec<Account>,
    categories: Vec<Category>,
    payees: Vec<Payee>,
    tags: Vec<Tag>,
    transactions: Vec<Transaction>,
    bill_plans: Vec<BillPlan>,
    bill_entries: Vec<BillScheduleEntry>,
    budgets: Budgets,
    inventory: Inventory,
    institutions: Vec<Institution>,
}

/// Every collection a [`Ledger`] is built from. A test fills only the collections it reads and
/// leaves the rest empty.
#[derive(Debug, Clone)]
pub struct LedgerParts {
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub payees: Vec<Payee>,
    pub tags: Vec<Tag>,
    pub transactions: Vec<Transaction>,
    pub bill_plans: Vec<BillPlan>,
    pub bill_entries: Vec<BillScheduleEntry>,
    pub budgets: Budgets,
    pub inventory: Inventory,
    pub institutions: Vec<Institution>,
}

impl Default for LedgerParts {
    /// Empty collections.
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            categories: Vec::new(),
            payees: Vec::new(),
            tags: Vec::new(),
            transactions: Vec::new(),
            bill_plans: Vec::new(),
            bill_entries: Vec::new(),
            budgets: Budgets::default(),
            inventory: Inventory::default(),
            institutions: Vec::new(),
        }
    }
}

impl Ledger {
    /// A Ledger over `parts`, with nothing recorded.
    pub fn from_parts(parts: LedgerParts) -> Self {
        let LedgerParts {
            accounts,
            categories,
            payees,
            tags,
            transactions,
            bill_plans,
            bill_entries,
            budgets,
            inventory,
            institutions,
        } = parts;
        Self {
            accounts,
            categories,
            payees,
            tags,
            transactions,
            bill_plans,
            bill_entries,
            budgets,
            inventory,
            institutions,
        }
    }

    /// The stub Ledger, anchored to today: each domain's stub seed, the Bills naming the
    /// Transactions `lib-transactions`' stub books for them. The Documents aren't the Ledger's: the
    /// stub's are [`Ledger::stub_documents`]. Nor are the Units: the Units store is their one
    /// home (#1052).
    pub fn stub() -> Self {
        Self::stub_on(Local::now().date_naive())
    }

    /// The stub Ledger anchored to `today`, so a test is not at the mercy of the wall clock.
    pub fn stub_on(today: NaiveDate) -> Self {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = seed::stub_transactions(&accounts, &categories, &payees, &tags, today);
        let bills = seed::stub_bills(&accounts, &categories, &payees, today);
        let inventory = default_inventory(today);
        let institutions = default_institutions();
        let budgets = default_budgets(
            &budget_accounts(&accounts),
            &budget_categories(&categories),
            today,
        );
        Self::from_parts(LedgerParts {
            accounts,
            categories,
            payees,
            tags,
            transactions,
            bill_plans: bills.plans,
            bill_entries: bills.entries,
            budgets,
            inventory,
            institutions,
        })
    }

    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub(crate) fn payees(&self) -> &[Payee] {
        &self.payees
    }

    pub(crate) fn tags(&self) -> &[Tag] {
        &self.tags
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn bill_plans(&self) -> &[BillPlan] {
        &self.bill_plans
    }

    pub fn bill_entries(&self) -> &[BillScheduleEntry] {
        &self.bill_entries
    }

    pub fn budgets(&self) -> &Budgets {
        &self.budgets
    }

    pub(crate) fn inventory(&self) -> &Inventory {
        &self.inventory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every id a Transaction names must resolve in the stub, or a View would render a dangling row.
    #[test]
    fn every_transaction_reference_resolves_in_the_stub() {
        let ledger = Ledger::stub();
        for transaction in ledger.transactions() {
            assert!(
                ledger
                    .accounts()
                    .iter()
                    .any(|a| a.id == transaction.account_id)
            );
            for split in &transaction.splits {
                assert!(
                    ledger
                        .categories()
                        .iter()
                        .any(|c| c.id == split.category_id)
                );
                if let Some(payee_id) = split.payee_id {
                    assert!(ledger.payees().iter().any(|p| p.id == payee_id));
                }
                for tag_id in &split.tag_ids {
                    assert!(ledger.tags().iter().any(|t| t.id == *tag_id));
                }
            }
        }
    }
}
