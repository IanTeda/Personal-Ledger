//! The Ledger aggregate: every domain collection, joined into one stub dataset.

mod account_commands;
mod bill_commands;
mod budget_commands;
#[cfg(test)]
mod budgets_seed_tests;
mod category_commands;
mod change;
mod document_commands;
mod hatch;
mod payee_commands;
pub mod query;
pub mod seed;
mod tag_commands;
mod transaction_commands;

pub use account_commands::{
    AccountDeleted, AddAccount, CountImportedTransactions, DeleteAccount, EditAccount,
};
pub use bill_commands::{
    AddBillPlan, EditBillPlan, MatchBill, PayBill, SkipBill, UnmatchBill, UnskipBill,
};
pub use budget_commands::{
    AddBudget, ArchiveBudget, CycleBudgetRollover, DuplicateBudget, EditBudget, FillBudget,
    RestoreBudget, SaveBudgetCell, SetBudgetAmount, SetDefaultBudget, StopBudgeting,
};
pub use category_commands::{
    AddCategory, CategoryDeleted, ChangeCategoryType, DeleteCategory, DeleteCategoryRefusal,
    EditCategory, MonthlyBudget, MoveCategory, RenameCategory, SaveCategoryRefusal,
};
pub use change::{Domain, LedgerChange, LedgerCommand, domains};
pub use document_commands::{
    AcceptAllDocuments, AcceptDocument, AddDocument, AddDocumentType, DocumentEdit,
    DropDocumentInventoryLinks, EditDocument, EditDocumentType, FileDocumentByHand,
    ImportDocuments, MoveDocumentType, RemoveDocumentType, SetDocumentFacts, SkipDocument,
    ToggleDocumentLink, UndoFiling, UnknownDocumentType,
};
pub use hatch::LedgerHatch;
pub use payee_commands::{
    AddPayee, DeletePayee, DeletePayeeRefusal, EditPayee, PayeeUse, SetPayeeActive,
    SetPayeeActiveRefusal,
};
pub use seed::{DEFAULT_SEED, HISTORY_DAYS, default_transactions};
pub use tag_commands::{AddTag, EditTag, MergeTags, RemoveTag};
pub use transaction_commands::{BookingPayee, CommitImport, ImportBooking, ImportCommitted};

use chrono::{Local, NaiveDate};
use lib_accounts::{Account, default_accounts};
use lib_bills::{BillPlan, BillScheduleEntry, default_bills};
use lib_budgets::{Budgets, default_budgets};
use lib_categories::{Category, default_categories};
use lib_documents::{
    Document, default_documents,
    types::{DocumentTypeRow, default_types, first_free_id},
};
use lib_institutions::{InstitutionRow, default_institutions};
use lib_inventory::{Inventory, default_inventory};
use lib_payees::{Payee, default_payees};
use lib_tags::{Tag, default_tags};
use lib_transactions::Transaction;
use lib_units::{PriceSourceRow, UnitRow, default_price_sources, default_units};

/// Every domain collection the Clients read, owned in one place. Every write is a
/// [`LedgerCommand`] that records what it changed, except the temporary
/// [`hatch`](Ledger::hatch) the migration onto commands still needs.
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
    /// The id the next added Document Type takes. It only counts up, so ids are never reused, and
    /// it goes when ids become UUIDv7 newtypes (ADR-0040).
    document_type_next_id: u32,
    inventory: Inventory,
    institutions: Vec<InstitutionRow>,
    units: Vec<UnitRow>,
    price_sources: Vec<PriceSourceRow>,
    /// The changes recorded since the last [`Ledger::drain_changes`].
    pending: Vec<LedgerChange>,
}

/// Every collection a [`Ledger`] is built from. A test fills only the collections it reads and
/// leaves the rest empty.
#[derive(Debug, Clone, Default)]
pub struct LedgerParts {
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub payees: Vec<Payee>,
    pub tags: Vec<Tag>,
    pub transactions: Vec<Transaction>,
    pub bill_plans: Vec<BillPlan>,
    pub bill_entries: Vec<BillScheduleEntry>,
    pub budgets: Budgets,
    pub documents: Vec<Document>,
    pub document_types: Vec<DocumentTypeRow>,
    pub inventory: Inventory,
    pub institutions: Vec<InstitutionRow>,
    pub units: Vec<UnitRow>,
    pub price_sources: Vec<PriceSourceRow>,
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
            documents,
            document_types,
            inventory,
            institutions,
            units,
            price_sources,
        } = parts;
        let document_type_next_id = first_free_id(&document_types);
        Self {
            accounts,
            categories,
            payees,
            tags,
            transactions,
            bill_plans,
            bill_entries,
            budgets,
            documents,
            document_types,
            document_type_next_id,
            inventory,
            institutions,
            units,
            price_sources,
            pending: Vec::new(),
        }
    }

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
        Self::from_parts(LedgerParts {
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
        })
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

    /// The changes recorded since the last drain, oldest first, for the store to hand to the
    /// repository (ADR-0039).
    pub fn drain_changes(&mut self) -> Vec<LedgerChange> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hatch write skips the change list, so the repository never sees it: that's why every
    /// write has to move onto a command before persistence is wired.
    #[test]
    #[expect(deprecated, reason = "tests the hatch itself")]
    fn a_hatch_write_records_no_change() {
        let mut ledger = Ledger::stub();
        ledger.hatch().payees.clear();
        assert!(ledger.payees().is_empty());
        assert!(ledger.drain_changes().is_empty());
    }

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
