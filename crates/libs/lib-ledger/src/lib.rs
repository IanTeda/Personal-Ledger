//! The Ledger aggregate: every domain collection, joined into one stub dataset.

mod account_commands;
mod bill_reads;
mod bills_seed;
#[cfg(test)]
mod bills_seed_tests;
mod budget_commands;
#[cfg(test)]
mod budgets_seed_tests;
mod category_commands;
mod change;
mod document_commands;
mod institution_commands;
mod institution_reads;
mod inventory_commands;
mod payee_reads;
pub mod query;
mod repository;
pub mod seed;
mod tag_commands;
mod transaction_commands;
mod unit_usage;

pub use account_commands::{
    AccountDeleted, AddAccount, CountImportedTransactions, DeleteAccount, EditAccount,
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
pub use institution_commands::AddInstitution;
pub use inventory_commands::{
    AddArea, AddLocation, EditArea, EditLocation, MoveArea, MoveAreaRefusal, RemoveArea,
    RemoveLocation,
};
pub use repository::{LedgerRepository, RepositoryError, StubRepository};
pub use seed::{DEFAULT_SEED, HISTORY_DAYS, default_transactions};
pub use tag_commands::{AddTag, EditTag, MergeTags, RemoveTag};
pub use transaction_commands::{BookingPayee, CommitImport, ImportBooking, ImportCommitted};

use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use lib_accounts::{Account, default_accounts};
use lib_bills::{BillPlan, BillScheduleEntry, default_bills};
use lib_budgets::{Budgets, default_budgets};
use lib_categories::{Category, default_categories};
use lib_core::{PriceSourceId, UnitId, UnitKind};
use lib_documents::{
    Document, default_documents,
    types::{DocumentTypeRow, default_types},
};
use lib_institutions::{Institution, default_institutions};
use lib_inventory::{Inventory, default_inventory};
use lib_payees::{Payee, default_payees};
use lib_tags::{Tag, default_tags};
use lib_transactions::Transaction;
use lib_units::{PriceSource, PriceSourceRecord, UnitRow};

/// Every domain collection the Clients read, owned in one place. Every write is a
/// [`LedgerCommand`] that records what it changed.
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
    institutions: Vec<Institution>,
    units: Vec<UnitRow>,
    /// The Ledger's one base Unit, which is also the default for new Accounts.
    base_unit: UnitId,
    /// Only the Price Sources a person changed: built-in overrides and Custom sources.
    price_sources: Vec<PriceSourceRecord>,
    /// The changes recorded since the last [`Ledger::drain_changes`].
    pending: Vec<LedgerChange>,
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
    pub documents: Vec<Document>,
    pub document_types: Vec<DocumentTypeRow>,
    pub inventory: Inventory,
    pub institutions: Vec<Institution>,
    pub units: Vec<UnitRow>,
    pub base_unit: UnitId,
    pub price_sources: Vec<PriceSourceRecord>,
}

impl Default for LedgerParts {
    /// Empty collections. `base_unit` is the stub's aud, which an empty `units` doesn't hold: a
    /// test that reads the base Unit supplies its own `units`.
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
            documents: Vec::new(),
            document_types: Vec::new(),
            inventory: Inventory::default(),
            institutions: Vec::new(),
            units: Vec::new(),
            base_unit: UnitId::SEED_AUD,
            price_sources: Vec::new(),
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
            documents,
            document_types,
            inventory,
            institutions,
            units,
            base_unit,
            price_sources,
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
            documents,
            document_types,
            inventory,
            institutions,
            units,
            base_unit,
            price_sources,
            pending: Vec::new(),
        }
    }

    /// The stub Ledger, anchored to today. Built in the Shell's order: Bills and Documents write
    /// their history into the Transactions, so Transactions are seeded before them and the rest
    /// follow their dependencies.
    pub fn stub() -> Self {
        Self::stub_at(Local::now().date_naive(), Utc::now())
    }

    /// The stub Ledger anchored to `today`, so a test is not at the mercy of the wall clock. Its
    /// "now", for the Units' last prices, is noon that day.
    pub fn stub_on(today: NaiveDate) -> Self {
        Self::stub_at(
            today,
            today.and_time(NaiveTime::MIN).and_utc() + chrono::Duration::hours(12),
        )
    }

    fn stub_at(today: NaiveDate, now: DateTime<Utc>) -> Self {
        let units = lib_units::fake::InMemoryPorts::stub(now);
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let mut transactions = default_transactions(&accounts, &categories, &payees, &tags, today);
        let bills = bills_seed::book_stub_bills(
            default_bills(
                &bill_reads::bill_accounts(&accounts),
                &bill_reads::bill_categories(&categories),
                &bill_reads::bill_payees(&payees),
                today,
            ),
            &mut transactions,
        );
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
            units: units.units.units,
            base_unit: units.base_unit.0,
            price_sources: Vec::new(),
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

    /// Replaces the read copy of the Transactions with what the Transactions port holds (#774's
    /// halfway shim, removed once every reader goes through that port), newest first as every
    /// reader takes them, since a port lists in no particular order. Returns whether anything
    /// changed, so a Client announces only a real change.
    pub fn refresh_transactions(&mut self, mut transactions: Vec<Transaction>) -> bool {
        sort_newest_first(&mut transactions);
        if self.transactions == transactions {
            return false;
        }
        self.transactions = transactions;
        true
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

    pub fn institutions(&self) -> &[Institution] {
        &self.institutions
    }

    pub fn units(&self) -> &[UnitRow] {
        &self.units
    }

    /// The Ledger's base Unit, which new Accounts default to.
    pub fn base_unit(&self) -> UnitId {
        self.base_unit
    }

    /// The base Unit's row, or `None` when it has since been removed.
    pub fn base_unit_row(&self) -> Option<&UnitRow> {
        self.unit(self.base_unit)
    }

    /// The Unit `id`, if it exists: how an Account's Unit reference becomes a code to show.
    pub fn unit(&self, id: UnitId) -> Option<&UnitRow> {
        self.units.iter().find(|unit| unit.id == id)
    }

    /// The code of Unit `id` for display, empty when the Unit is gone.
    pub fn unit_code(&self, id: UnitId) -> &str {
        self.unit(id).map_or("", |unit| unit.code.as_str())
    }

    /// The Units sorted by code, ignoring case.
    pub fn units_by_code(&self) -> Vec<&UnitRow> {
        lib_units::units_by_code(&self.units)
    }

    /// The stored Price Source rows, as the repository receives them.
    pub fn price_source_records(&self) -> &[PriceSourceRecord] {
        &self.price_sources
    }

    /// Every Price Source, Manual entry and the built-ins included, sorted by current name
    /// ignoring case. `manual_name` is Manual entry's name in the Client's Locale.
    pub fn price_sources<'a>(&'a self, manual_name: &'a str) -> Vec<PriceSource<'a>> {
        lib_units::price_sources(&self.price_sources, manual_name)
    }

    /// The Price Source `id`, if it exists.
    pub fn price_source<'a>(
        &'a self,
        manual_name: &'a str,
        id: PriceSourceId,
    ) -> Option<PriceSource<'a>> {
        lib_units::price_source(&self.price_sources, manual_name, id)
    }

    /// The Price Sources able to price a Unit of `kind`.
    pub fn sources_for_kind<'a>(
        &'a self,
        manual_name: &'a str,
        kind: UnitKind,
    ) -> Vec<PriceSource<'a>> {
        lib_units::sources_for_kind(&self.price_sources, manual_name, kind)
    }

    /// When Price Source `id` last fetched a price, or `None` when it has not fetched yet.
    pub fn last_fetched(&self, id: PriceSourceId) -> Option<DateTime<Utc>> {
        lib_units::last_fetched(&self.units, id)
    }

    /// The changes recorded since the last drain, oldest first, for the store to hand to the
    /// repository (ADR-0039).
    pub fn drain_changes(&mut self) -> Vec<LedgerChange> {
        std::mem::take(&mut self.pending)
    }
}

/// Newest first, the later id first on one day: the order the Transactions readers take, which an
/// Import's commit keeps too.
fn sort_newest_first(transactions: &mut [Transaction]) {
    transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
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

    #[test]
    fn refreshing_the_transactions_read_copy_reports_only_a_real_change() {
        let mut ledger = Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).unwrap());
        let mut transactions = ledger.transactions().to_vec();
        transactions.reverse();
        ledger.refresh_transactions(transactions.clone());

        assert!(
            !ledger.refresh_transactions(transactions.clone()),
            "the same rows in another order are no change"
        );
        assert!(
            ledger
                .transactions()
                .windows(2)
                .all(|pair| (pair[0].date, pair[0].id) > (pair[1].date, pair[1].id)),
            "newest first"
        );

        let removed = transactions.remove(0);
        assert!(ledger.refresh_transactions(transactions));
        assert!(ledger.transactions().iter().all(|t| t.id != removed.id));
    }
}
