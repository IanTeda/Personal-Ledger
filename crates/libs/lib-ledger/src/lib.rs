//! The Ledger aggregate: every domain collection, joined into one stub dataset.

#[cfg(test)]
mod budgets_seed_tests;
pub mod query;
pub mod seed;

pub use seed::{DEFAULT_SEED, HISTORY_DAYS, default_transactions};

use chrono::{Local, NaiveDate};
use lib_accounts::{Account, default_accounts};
use lib_bills::{BillPlan, BillScheduleEntry, default_bills};
use lib_budgets::{Budgets, default_budgets};
use lib_categories::{Category, default_categories};
use lib_documents::{
    Document, default_documents,
    types::{DocumentTypeRow, default_types},
};
use lib_institutions::{InstitutionRow, default_institutions};
use lib_inventory::{Inventory, default_inventory};
use lib_payees::{Payee, default_payees};
use lib_tags::{Tag, default_tags};
use lib_transactions::Transaction;
use lib_units::{PriceSourceRow, UnitRow, default_price_sources, default_units};

/// Every domain collection the Clients read, owned in one place. Built only by [`Ledger::stub`]
/// for now; the commands that change it follow the Ledger prototype.
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
    documents: Vec<Document>,
    document_types: Vec<DocumentTypeRow>,
    inventory: Inventory,
    institutions: Vec<InstitutionRow>,
    units: Vec<UnitRow>,
    price_sources: Vec<PriceSourceRow>,
}

impl Ledger {
    /// The stub Ledger, anchored to today. Built in the Shell's order: Bills and Documents write
    /// their history into the Transactions, so Transactions are seeded before them and the rest
    /// follow their dependencies.
    pub fn stub() -> Self {
        Self::stub_on(Local::now().date_naive())
    }

    /// The stub Ledger anchored to `today`, so a test is not at the mercy of the wall clock.
    pub fn stub_on(today: NaiveDate) -> Self {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let mut transactions = default_transactions(&accounts, &categories, &payees, &tags, today);
        let bills = default_bills(&accounts, &categories, &payees, &mut transactions, today);
        let inventory = default_inventory(today);
        let documents = default_documents(
            &accounts,
            &categories,
            &payees,
            &bills.plans,
            &inventory,
            &mut transactions,
            today,
        );
        let budgets = default_budgets(&accounts, &categories, today);
        Self {
            accounts,
            categories,
            payees,
            tags,
            transactions,
            bill_plans: bills.plans,
            bill_entries: bills.entries,
            budgets,
            documents: documents.documents,
            document_types: default_types(),
            inventory,
            institutions: default_institutions(),
            units: default_units(),
            price_sources: default_price_sources(),
        }
    }

    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn payees(&self) -> &[Payee] {
        &self.payees
    }

    pub fn tags(&self) -> &[Tag] {
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

    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    pub fn document_types(&self) -> &[DocumentTypeRow] {
        &self.document_types
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    pub fn institutions(&self) -> &[InstitutionRow] {
        &self.institutions
    }

    pub fn units(&self) -> &[UnitRow] {
        &self.units
    }

    pub fn price_sources(&self) -> &[PriceSourceRow] {
        &self.price_sources
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
