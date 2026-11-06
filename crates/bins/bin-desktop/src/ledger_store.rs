//! The Desktop's announcer of Ledger writes. [`LedgerStore`] holds no rows (#1056): every domain
//! is written by its module, through its ports or a cross-domain module's port, and the store
//! emits a [`LedgerChanged`] naming the [`Domain`]s that changed, which [`notify_on`] turns into a
//! repaint for the Entities reading them. It writes no domain outside its `#[doc(hidden)]` test
//! hooks (`*_for_test`), and still hands out the shared stores and the stored reads until they
//! move to the Open Ledger (#1057).
//!
//! The Institutions, Accounts and Categories have no read copy (#1053, #1054, #1055): their
//! modules announce their writes through [`LedgerStore::announce_institutions`],
//! [`LedgerStore::announce_accounts`] and [`LedgerStore::refresh_categories`], which announces
//! from the Categories store's write count. The Tags, Payees, Transactions, Bills and
//! Budgets have no read copy (#1003, #1005, #1028): their modules and the cross-domain modules
//! write their stores and announce the change through [`LedgerStore::announce_tags`],
//! [`LedgerStore::announce_payees`], [`LedgerStore::announce_transactions`],
//! [`LedgerStore::announce_bills`] and [`LedgerStore::announce_budgets`], which is the Dashboard's
//! and the other pages' redraw trigger. The stub's rows are `lib_database::FakeStores::stub`'s,
//! which the in-memory test stores are built from. The Documents and Document Types have no read
//! copy (#968): their modules read the Documents store, and the other domains' rows through their
//! stores ([`LedgerStore::document_reads`]).
//!
//! The store holds no commit: a module's writes are committed by the input they ran in
//! (`crate::input::send`).

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui::{Context, Entity, EventEmitter, Subscription};
use lib_database::{
    PayeesStore, SharedBudgetsStore, SharedCategoriesStore, SharedDocumentsStore,
    SharedInstitutionsStore, SharedInventoryStore, SharedTagsStore, SharedTransactionsStore,
    SharedUnitsStore, StoredCategoryRows, StoredDocumentReads, StoredInventoryReads,
    StoredPayeeReads,
};
use lib_institutions::InstitutionsPorts as _;

use crate::accounts::ports::DesktopPorts as DesktopAccountsPorts;
use crate::bills::ports::{DesktopPorts as DesktopBillsPorts, LedgerBackedPorts};
use crate::institutions::ports::DesktopPorts as DesktopInstitutionsPorts;
use crate::payees::ports::DesktopPorts as DesktopPayeesPorts;
use crate::write_count::WriteCount;

/// What `shell::router::build` hands the Ledger: the stores and ports its modules and the
/// cross-domain modules write, which the Ledger keeps read copies of, lends to the readers not yet
/// on their own ports, and reloads after a failed write. It writes none of them (#1006).
pub struct LedgerSetup {
    pub institutions: DesktopInstitutionsPorts,
    /// The Institutions store under the Institutions ports, which the Accounts' reads hold too.
    pub institutions_store: SharedInstitutionsStore,
    pub accounts: DesktopAccountsPorts,
    pub categories: SharedCategoriesStore,
    /// The writes made to the Categories store, which the Ledger announces from.
    pub categories_writes: WriteCount,
    pub tags: SharedTagsStore,
    pub payees: DesktopPayeesPorts,
    pub transactions: SharedTransactionsStore,
    pub bills: DesktopBillsPorts,
    pub budgets: SharedBudgetsStore,
    pub inventory: SharedInventoryStore,
    pub documents: SharedDocumentsStore,
    /// The Units store the Units ports share and the Accounts, Institutions, Bills, Tags and
    /// Inventory reads take Units from: the Ledger file's, or the stub's in memory in a test.
    pub units: SharedUnitsStore,
}

impl LedgerSetup {
    /// The stub's Institutions, Accounts, Categories, Payees, Transactions and Bills as of `today`
    /// (`FakeStores::stub`), in memory, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(today: chrono::NaiveDate) -> Self {
        let stub = lib_database::FakeStores::stub(today);
        let (categories, categories_writes) = crate::categories::ports::in_memory_counted();
        let budgets = crate::budgets::ports::in_memory(&stub);
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(stub.transactions.clone()));
        // The Units' last prices are as of noon that day, as the stub Ledger's were.
        let units = crate::bills::ports::in_memory_units(
            today.and_time(chrono::NaiveTime::MIN).and_utc() + chrono::Duration::hours(12),
        );
        let institutions_store = crate::institutions::ports::in_memory_store();
        let accounts =
            crate::accounts::LedgerBackedPorts::in_memory(lib_database::StoredAccountReads {
                transactions: transactions.clone(),
                budgets: budgets.clone(),
                units: units.clone(),
                institutions: institutions_store.clone(),
            });
        let bills_store = crate::bills::ports::in_memory_store(&stub);
        let payees = crate::payees::LedgerBackedPorts::in_memory(StoredPayeeReads {
            transactions: transactions.clone(),
            bills: bills_store.clone(),
            categories: categories.clone(),
        });
        let bills = LedgerBackedPorts::shared(
            bills_store,
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
            units.clone(),
        );
        Self {
            budgets,
            documents: Rc::new(RefCell::new(stub.documents)),
            tags: crate::tags::ports::in_memory(),
            inventory: crate::inventory::ports::in_memory(today),
            institutions: crate::institutions::LedgerBackedPorts::shared(
                institutions_store.clone(),
                accounts.clone(),
                units.clone(),
            ),
            institutions_store,
            bills,
            transactions,
            accounts,
            categories,
            categories_writes,
            payees,
            units,
        }
    }
}

/// A part of the Ledger a write can touch: what [`LedgerChanged`] carries when a module has
/// written its store, so a reader can skip a change that isn't its own. Distinct from
/// `lib_database`'s writer `Domain`, which names the adapters a failed write reloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Accounts,
    Categories,
    Payees,
    Tags,
    Transactions,
    Bills,
    Budgets,
    Documents,
    DocumentTypes,
    Inventory,
    Institutions,
    Units,
}

/// Emitted after a write lands, naming every domain it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerChanged {
    pub what: BTreeSet<Domain>,
}

pub struct LedgerStore {
    institutions: DesktopInstitutionsPorts,
    accounts: DesktopAccountsPorts,
    categories: SharedCategoriesStore,
    /// The writes made to the Categories store, and the count the last announcement saw.
    categories_writes: WriteCount,
    categories_announced: u64,
    tags: SharedTagsStore,
    payees: DesktopPayeesPorts,
    transactions: SharedTransactionsStore,
    bills: DesktopBillsPorts,
    budgets: SharedBudgetsStore,
    inventory: SharedInventoryStore,
    documents: SharedDocumentsStore,
    /// The other domains as Documents reads them, through the stores above (#963).
    document_reads: StoredDocumentReads,
    /// The Units and Documents the Inventory Dialogs quote (#1014).
    inventory_reads: StoredInventoryReads,
    /// Whether a change was announced since the Shell last read the rail's counts.
    announced: bool,
    /// How many changes were announced, for a test that asserts a keystroke announced none.
    announcements: usize,
}

impl EventEmitter<LedgerChanged> for LedgerStore {}

impl LedgerStore {
    /// `stub`'s Transactions, Bills and Budgets in memory as the ports, beside the stub's
    /// Institutions, Accounts, Categories and Payees, for a test that needs no storage.
    pub fn new(stub: &lib_database::FakeStores) -> Self {
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(stub.transactions.clone()));
        let (categories, categories_writes) = crate::categories::ports::in_memory_counted();
        let bills_store = crate::bills::ports::in_memory_store(stub);
        let payees = crate::payees::LedgerBackedPorts::in_memory(StoredPayeeReads {
            transactions: transactions.clone(),
            bills: bills_store.clone(),
            categories: categories.clone(),
        });
        let budgets = crate::budgets::ports::in_memory(stub);
        let units = crate::bills::ports::in_memory_units(chrono::Utc::now());
        let institutions_store = crate::institutions::ports::in_memory_store();
        let accounts =
            crate::accounts::LedgerBackedPorts::in_memory(lib_database::StoredAccountReads {
                transactions: transactions.clone(),
                budgets: budgets.clone(),
                units: units.clone(),
                institutions: institutions_store.clone(),
            });
        let bills = LedgerBackedPorts::shared(
            bills_store,
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
            units.clone(),
        );
        let institutions = crate::institutions::LedgerBackedPorts::shared(
            institutions_store,
            accounts.clone(),
            units.clone(),
        );
        let inventory = crate::inventory::ports::in_memory(chrono::Local::now().date_naive());
        let documents: SharedDocumentsStore =
            Rc::new(RefCell::new(lib_documents::fake::InMemoryDocuments {
                documents: Vec::new(),
                types: lib_documents::types::default_types(),
            }));
        let inventory_reads = StoredInventoryReads {
            units,
            inventory: inventory.clone(),
            documents: documents.clone(),
        };
        let document_reads = StoredDocumentReads {
            transactions: transactions.clone(),
            payees: payees.clone(),
            accounts: accounts.clone(),
            institutions: institutions.clone(),
            bills: bills.borrow().store(),
            inventory: inventory.clone(),
        };
        Self {
            institutions,
            accounts,
            categories,
            categories_writes,
            categories_announced: 0,
            tags: crate::tags::ports::in_memory(),
            inventory,
            documents,
            document_reads,
            inventory_reads,
            payees,
            transactions,
            bills,
            budgets,
            announced: false,
            announcements: 0,
        }
    }

    /// The Institutions, Accounts, Categories, Payees, Transactions and Bills read from `setup`'s
    /// stores, which their writes go to.
    pub fn with_storage(setup: LedgerSetup) -> Self {
        let document_reads = StoredDocumentReads {
            transactions: setup.transactions.clone(),
            payees: setup.payees.clone(),
            accounts: setup.accounts.clone(),
            institutions: setup.institutions.clone(),
            bills: setup.bills.borrow().store(),
            inventory: setup.inventory.clone(),
        };
        let inventory_reads = StoredInventoryReads {
            units: setup.units.clone(),
            inventory: setup.inventory.clone(),
            documents: setup.documents.clone(),
        };
        Self {
            institutions: setup.institutions,
            accounts: setup.accounts,
            categories: setup.categories,
            categories_writes: setup.categories_writes,
            categories_announced: 0,
            tags: setup.tags,
            payees: setup.payees,
            transactions: setup.transactions,
            bills: setup.bills,
            budgets: setup.budgets,
            inventory: setup.inventory,
            documents: setup.documents,
            document_reads,
            inventory_reads,
            announced: false,
            announcements: 0,
        }
    }

    /// The other domains' stores as `lib_inventory`' reads, which the Inventory Dialogs hold.
    pub fn inventory_reads(&self) -> StoredInventoryReads {
        self.inventory_reads.clone()
    }

    /// The other domains' stores as `lib_documents`' reads, which the Documents page's Presenter
    /// holds.
    pub fn document_reads(&self) -> StoredDocumentReads {
        self.document_reads.clone()
    }

    /// The rows naming a Category in the Transactions, Bills and Payees stores, which the
    /// Categories ports count and hand over through (#1034): the one selection Category delete
    /// reads too.
    pub fn category_rows(&self) -> StoredCategoryRows {
        StoredCategoryRows {
            transactions: self.transactions.clone(),
            bills: self.bills.borrow().store(),
            payees: self.payees.clone(),
        }
    }

    /// Adds a Location priced in `unit` to the Inventory store, so a test can delete a Unit one
    /// names (#885).
    #[doc(hidden)]
    pub fn add_location_in_for_test(&mut self, unit: lib_core::UnitId, cx: &mut Context<'_, Self>) {
        let mut scratch = lib_inventory::Inventory::default();
        let draft = lib_inventory::LocationDraft {
            name: "Beach shack".to_string(),
            ..lib_inventory::LocationDraft::default()
        };
        if scratch.add_location(&draft, unit).is_err() {
            return;
        }
        let Some(location) = scratch.locations.pop() else {
            return;
        };
        self.inventory
            .borrow_mut()
            .inventory_mut()
            .upsert_location(location);
        self.announce(cx, BTreeSet::from([Domain::Units, Domain::Inventory]));
    }

    /// Removes every Location from the Inventory store through the Interactor, as the Remove
    /// location Dialog does.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&mut self, cx: &mut Context<'_, Self>) {
        let ids: Vec<lib_core::LocationId> = self
            .inventory
            .borrow()
            .inventory()
            .locations()
            .iter()
            .map(|location| location.id)
            .collect();
        for id in ids {
            let removed = lib_inventory::interactor::RemoveLocation(id)
                .run(self.inventory.borrow_mut().inventory_mut());
            assert!(
                removed.is_ok(),
                "a Location listed a moment ago must still be removable"
            );
        }
        self.announce_inventory(cx);
    }

    /// Writes an empty default Budget in `unit` straight to the Budgets store, which then names
    /// the Unit.
    #[doc(hidden)]
    pub fn add_budget_in_for_test(&mut self, unit: lib_core::UnitId, cx: &mut Context<'_, Self>) {
        let budget = lib_budgets::personal_spending(unit, &[]);
        self.budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);
        self.announce(cx, BTreeSet::from([Domain::Units, Domain::Budgets]));
    }

    /// Writes a Payee called `name` straight to the Payees store, so a test can meet the refusal
    /// of an Import that creates the same name.
    #[doc(hidden)]
    pub fn add_payee_for_test(&mut self, name: &str, cx: &mut Context<'_, Self>) {
        PayeesStore::payees_mut(&mut *self.payees.borrow_mut()).upsert_payee(lib_payees::Payee {
            id: lib_core::PayeeId::new(),
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
            is_active: true,
        });
        self.announce_payees(cx);
    }

    /// Gives Institution `id` Unit `unit` as its Usual unit, so a test can delete a Unit an
    /// Institution names.
    pub fn name_usual_unit_for_test(
        &mut self,
        id: lib_core::InstitutionId,
        unit: lib_core::UnitId,
        cx: &mut Context<'_, Self>,
    ) {
        {
            let mut ports = self.institutions.borrow_mut();
            let Some(mut institution) = ports.institutions().institution(id) else {
                return;
            };
            institution.usual_unit = Some(unit);
            ports.institutions_mut().upsert_institution(institution);
        }
        self.announce_institutions(cx);
    }

    /// The Budgets store, which the Categories Monthly budget writes and the usage counts read.
    pub fn budgets_store(&self) -> &SharedBudgetsStore {
        &self.budgets
    }

    /// The Payees ports, for a write failure's rollback to read back (ADR-0046).
    pub fn payees_ports(&self) -> &DesktopPayeesPorts {
        &self.payees
    }

    /// The Tags store, for a write failure's rollback to read back (ADR-0046).
    pub fn tags_store(&self) -> &SharedTagsStore {
        &self.tags
    }

    /// The Transactions store, which the Categories handover and a test helper write.
    pub fn transactions_store(&self) -> &SharedTransactionsStore {
        &self.transactions
    }

    /// The Bills store under the Bills ports, which the Categories handover writes.
    pub fn bills_store(&self) -> lib_database::SharedBillsStore {
        self.bills.borrow().store()
    }

    /// The Categories store's tree as the Budgets rules read it, for the Categories Monthly budget's
    /// save.
    pub fn budget_categories(&self) -> Vec<lib_budgets::BudgetCategory> {
        use lib_budgets::BudgetCategories as _;
        lib_database::StoredBudgetCategories {
            categories: self.categories.clone(),
        }
        .budget_categories()
    }

    /// Announces `Domain::Units`, after the Units module, Unit delete, the Ledger page's base
    /// Unit or a rollback wrote the Units store, so the Accounts, Inventory, Bills, Budgets,
    /// Transactions and the Dashboard redraw (#1052).
    pub fn announce_units(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Units]));
    }

    /// The Accounts ports, for a write failure's rollback to reload (ADR-0046).
    pub fn accounts_ports(&self) -> &DesktopAccountsPorts {
        &self.accounts
    }

    /// Announces `Domain::Accounts`, after the Accounts page or Account delete wrote the Accounts
    /// store, or a rollback reloaded it, so every reader redraws (#1054).
    pub fn announce_accounts(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Accounts]));
    }

    /// The Institutions ports, for a write failure's rollback to reload (ADR-0046).
    pub fn institutions_ports(&self) -> &DesktopInstitutionsPorts {
        &self.institutions
    }

    /// Announces `Domain::Institutions`, after the Institutions module, Unit delete or a rollback
    /// wrote the Institutions store, so the Institutions and Accounts pages redraw (#1053).
    pub fn announce_institutions(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Institutions]));
    }

    /// The Categories store, for a write failure's rollback to reload (ADR-0046).
    pub fn categories_store(&self) -> &SharedCategoriesStore {
        &self.categories
    }

    /// Announces `Domain::Categories`, after a rollback reloaded the store, so every reader
    /// redraws.
    pub fn announce_categories(&mut self, cx: &mut Context<'_, Self>) {
        self.categories_announced = self.categories_writes.get();
        self.announce(cx, BTreeSet::from([Domain::Categories]));
    }

    /// Puts the Transactions back as stored, after a write that named them failed, announcing
    /// `Domain::Transactions` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the store is left as it was.
    pub fn reload_transactions(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.transactions.borrow_mut().reload()?;
        self.announce(cx, BTreeSet::from([Domain::Transactions]));
        Ok(())
    }

    /// Puts the Bill Plans and the Bill Schedule back as stored, after a write that named them
    /// failed, announcing `Domain::Bills` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the ports are left as they were.
    pub fn reload_bills(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.bills.borrow_mut().reload()?;
        self.announce(cx, BTreeSet::from([Domain::Bills]));
        Ok(())
    }

    /// Puts the Budgets back as stored, after a write that named them failed, announcing
    /// `Domain::Budgets` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the store is left as it was.
    pub fn reload_budgets(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.budgets.borrow_mut().reload()?;
        self.announce(cx, BTreeSet::from([Domain::Budgets]));
        Ok(())
    }

    /// Puts the Inventory back as stored, after a write that named it failed, announcing
    /// `Domain::Inventory` so every reader redraws (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading it back; the store is left as it was.
    pub fn reload_inventory(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.inventory.borrow_mut().reload()?;
        self.announce(cx, BTreeSet::from([Domain::Inventory]));
        Ok(())
    }

    /// Puts the Documents and Document Types back as stored, after a write that named them failed,
    /// announcing `Domain::Documents` and `Domain::DocumentTypes` so every reader redraws
    /// (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the store is left as it was.
    pub fn reload_documents(&mut self, cx: &mut Context<'_, Self>) -> lib_database::Result<()> {
        self.documents.borrow_mut().reload()?;
        self.announce_documents(cx);
        Ok(())
    }

    /// Announces `Domain::Documents` and `Domain::DocumentTypes`, after the Document Types module
    /// wrote the store, so the Documents View redraws.
    pub fn announce_documents(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(
            cx,
            BTreeSet::from([Domain::Documents, Domain::DocumentTypes]),
        );
    }

    /// Announces `Domain::Payees`, after the Payees module, Category delete, the Import or a
    /// rollback wrote the Payees store, so Transactions, Bills, Budgets and Documents redraw.
    pub fn announce_payees(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Payees]));
    }

    /// Announces `Domain::Categories` when the Categories store was written since the last
    /// announcement, after the Categories module or Category delete wrote it.
    pub fn refresh_categories(&mut self, cx: &mut Context<'_, Self>) {
        if self.categories_writes.get() != self.categories_announced {
            self.announce_categories(cx);
        }
    }

    /// Announces `Domain::Tags`, after the Tags page, Tag remove or merge, or a rollback wrote the
    /// Tags store, so the Tags page, the Transactions and the Shell redraw.
    pub fn announce_tags(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Tags]));
    }

    /// Announces `Domain::Inventory`, after the Inventory page wrote the store, so the Documents
    /// and the Inventory page redraw.
    pub fn announce_inventory(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Inventory]));
    }

    /// Announces `Domain::Bills`, after the Bills module, Bill Payments or Account delete wrote
    /// the Bills store, so the Dashboard, Bills, Budgets and Documents redraw (#1028).
    pub fn announce_bills(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Bills]));
    }

    /// Announces `Domain::Budgets`, after the Budgets module or the Categories Monthly budget
    /// wrote the Budgets store, so the Dashboard and Budgets redraw (#1028).
    pub fn announce_budgets(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Budgets]));
    }

    /// Announces `Domain::Transactions`, after Bill Payments, Account delete, the Import, Tag
    /// remove or merge, or Category delete wrote the Transactions store, so the Dashboard, the
    /// Transactions, Bills, Budgets, Documents and Tags redraw (#1028).
    pub fn announce_transactions(&mut self, cx: &mut Context<'_, Self>) {
        self.announce(cx, BTreeSet::from([Domain::Transactions]));
    }

    /// Tells every reader of `what` that it changed, in one event however many domains it names,
    /// and marks the rail's counts as needing a fresh read ([`Self::take_announced`]).
    pub(crate) fn announce(&mut self, cx: &mut Context<'_, Self>, what: BTreeSet<Domain>) {
        self.announced = true;
        self.announcements += 1;
        cx.emit(LedgerChanged { what });
        cx.notify();
    }

    /// Whether anything was announced since the last ask, so the Shell reads the rail's counts
    /// afresh only after a change rather than after every input.
    /// How many changes were announced so far, for a test.
    #[doc(hidden)]
    pub fn announcements(&self) -> usize {
        self.announcements
    }

    pub(crate) fn take_announced(&mut self) -> bool {
        std::mem::take(&mut self.announced)
    }
}

/// Repaints the subscribing Entity whenever a landed write touches `domain`: how an Entity that
/// reads the Ledger straight from `ledger` follows the rows it shows (ADR-0038).
pub fn notify_on<T: 'static>(
    ledger: &Entity<LedgerStore>,
    domain: Domain,
    cx: &mut Context<'_, T>,
) -> Subscription {
    cx.subscribe(ledger, move |_, _, event: &LedgerChanged, cx| {
        if event.what.contains(&domain) {
            cx.notify();
        }
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_database::{Commit, PendingWrites};

    use super::*;
    use lib_bills::BillsPorts as _;

    /// The Desktop's start-up storage on a Ledger file in `dir`, as of `today`.
    fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> lib_database::Storage {
        lib_database::open_storage(&dir.join("ledger.pldb"), today, "Personal")
            .expect("the Ledger opens")
    }

    /// A store on `storage`, with the pending writes the test commits, as an input's `send` would.
    fn on_storage(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (Entity<LedgerStore>, PendingWrites) {
        let on = on_storage_with_bills(cx, storage);
        (on.store, on.pending)
    }

    /// A store on the Ledger file, with the ports the modules that write beside it hold.
    struct OnStorage {
        store: Entity<LedgerStore>,
        /// The Bills ports the store shares, which the Bills module writes.
        bills: DesktopBillsPorts,
        /// The payments port over the Transactions and Bills stores, the Bill Payments module's.
        payments: lib_bill_payments::SharedPayments,
        /// The deletion port over the Accounts, Transactions and Bills stores, the Account
        /// Deletion module's.
        deletion: lib_account_deletion::SharedDeletion,
        /// The deletion port over the Categories, Transactions, Bills, Payees and Budgets stores,
        /// the Category Deletion module's.
        category_deletion: lib_category_deletion::SharedCategoryDeletion,
        pending: PendingWrites,
    }

    /// [`on_storage`] with the ports the Bills, Bill Payments, Account Deletion and Category
    /// Deletion modules write.
    fn on_storage_with_bills(cx: &mut TestAppContext, storage: lib_database::Storage) -> OnStorage {
        let pending = storage.pending;
        let (categories, categories_writes) =
            crate::categories::ports::counted(Box::new(storage.categories));
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(storage.transactions));
        let bills_store: lib_database::SharedBillsStore = Rc::new(RefCell::new(storage.bills));
        let payees = crate::payees::LedgerBackedPorts::shared(
            Box::new(storage.payees),
            StoredPayeeReads {
                transactions: transactions.clone(),
                bills: bills_store.clone(),
                categories: categories.clone(),
            },
        );
        let budgets: SharedBudgetsStore = Rc::new(RefCell::new(storage.budgets));
        let units: SharedUnitsStore = Rc::new(RefCell::new(storage.units));
        let institutions_store: SharedInstitutionsStore =
            Rc::new(RefCell::new(storage.institutions));
        let accounts = crate::accounts::LedgerBackedPorts::shared(
            Box::new(storage.accounts),
            lib_database::StoredAccountReads {
                transactions: transactions.clone(),
                budgets: budgets.clone(),
                units: units.clone(),
                institutions: institutions_store.clone(),
            },
        );
        let institutions = crate::institutions::LedgerBackedPorts::shared(
            institutions_store.clone(),
            accounts.clone(),
            units.clone(),
        );
        let bills = LedgerBackedPorts::shared(
            bills_store,
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
            units.clone(),
        );
        let payments = crate::bill_payments::payments(transactions.clone(), &bills);
        let deletion =
            crate::account_deletion::deletion(accounts.clone(), transactions.clone(), &bills);
        let category_deletion = crate::category_deletion::DeletionStores::new(
            categories.clone(),
            transactions.clone(),
            &bills,
            payees.clone(),
            budgets.clone(),
        )
        .deletion;
        let store = cx.new(|_| {
            LedgerStore::with_storage(LedgerSetup {
                institutions,
                institutions_store,
                accounts,
                categories,
                categories_writes,
                tags: Rc::new(RefCell::new(storage.tags)),
                payees,
                transactions,
                bills: bills.clone(),
                budgets,
                inventory: Rc::new(RefCell::new(storage.inventory)),
                documents: Rc::new(RefCell::new(storage.documents)),
                units,
            })
        });
        OnStorage {
            store,
            bills,
            payments,
            deletion,
            category_deletion,
            pending,
        }
    }

    /// Deletes Account `id` through the Account Deletion module's port, then brings the read
    /// copies in line, as its Dialog's confirm does.
    fn delete_account(
        cx: &mut TestAppContext,
        on: &OnStorage,
        id: lib_core::AccountId,
    ) -> lib_account_deletion::Result<lib_account_deletion::AccountDeleted> {
        let deleted = lib_account_deletion::interactor::DeleteAccount { id }
            .run(&mut *on.deletion.borrow_mut());
        cx.update(|cx| crate::account_deletion::refresh(&on.store, cx));
        deleted
    }

    #[gpui::test]
    fn a_deleted_accounts_transactions_survive_a_restart(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let store = on.store.clone();
        let wallet = lib_accounts::default_accounts()
            .into_iter()
            .find(|account| account.name == "Wallet")
            .expect("a stub Account")
            .id;

        delete_account(cx, &on, wallet).expect("nothing pays from the Wallet");
        on.pending.commit();
        let before = store.read_with(cx, |store, _| transactions_of(store));
        crate::quit(&writer);
        drop(store);
        drop(on);
        drop(writer);

        // The next day, so a re-seed would show as shifted dates.
        let (reopened, _) = on_storage(
            cx,
            storage_in(dir.path(), today.succ_opt().expect("a date")),
        );
        let after = reopened.read_with(cx, |store, _| transactions_of(store));

        assert_eq!(after, before);
        assert!(
            after
                .iter()
                .all(|transaction| transaction.account_id != wallet)
        );
    }

    /// Category delete, through the Category Deletion module's port, re-points the Splits and Bill
    /// Plans to an Uncategorised it creates and clears the Payee defaults before the delete, so the
    /// file's foreign keys let it through, and a restart reads the result back.
    #[gpui::test]
    fn a_deleted_categorys_rows_survive_a_restart(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let OnStorage {
            store,
            category_deletion,
            pending,
            ..
        } = on_storage_with_bills(cx, storage);
        let gone = store.read_with(cx, |store, _| {
            let payees = payees_of(store);
            categories_of(store)
                .iter()
                .map(|category| category.id)
                .find(|&id| {
                    plans_of(store).iter().any(|plan| plan.category_id == id)
                        && payees.iter().any(|p| p.default_category == Some(id))
                        && transactions_of(store)
                            .iter()
                            .any(|t| t.splits.iter().any(|s| s.category_id == id))
                })
                .expect("a stub Category with Splits, a Bill Plan and a Payee default")
        });

        let deleted = lib_category_deletion::interactor::DeleteCategory { id: gone }
            .run(&mut *category_deletion.borrow_mut())
            .expect("a leaf");
        // The port holds the stores; let them go before the restart reopens the file.
        drop(category_deletion);
        assert!(deleted.splits_moved > 0 && deleted.bill_plans_moved > 0);
        assert!(deleted.payee_defaults_cleared > 0);
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        reopened.read_with(cx, |store, _| {
            let categories = categories_of(store);
            assert!(categories.iter().all(|c| c.id != gone));
            let home = lib_categories::find_by_name(&categories, "Uncategorised")
                .expect("the delete's Uncategorised is stored");
            let transactions = transactions_of(store);
            let splits = transactions.iter().flat_map(|t| &t.splits);
            assert!(splits.clone().all(|s| s.category_id != gone));
            assert_eq!(
                splits.filter(|s| s.category_id == home).count(),
                deleted.splits_moved
            );
            assert!(plans_of(store).iter().all(|p| p.category_id != gone));
            assert!(
                payees_of(store)
                    .iter()
                    .all(|p| p.default_category != Some(gone))
            );
        });
    }

    /// The Categories as the Categories store holds them: the Ledger keeps no read copy (#1055).
    fn categories_of(store: &LedgerStore) -> Vec<lib_categories::Category> {
        store.categories_store().borrow().categories().categories()
    }

    /// The Payees as the Payees store holds them: the Ledger keeps no read copy (#1005).
    fn payees_of(store: &LedgerStore) -> Vec<lib_payees::Payee> {
        PayeesStore::payees(&*store.payees_ports().borrow()).payees()
    }

    /// The Bill Plans and the Bill Schedule as the Bills ports hold them, the entries by due date
    /// and then Plan, since a port lists them in no particular order.
    fn bills_of(
        cx: &mut TestAppContext,
        store: &Entity<LedgerStore>,
    ) -> (Vec<lib_bills::BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
        store.read_with(cx, |store, _| (plans_of(store), entries_of(store)))
    }

    /// The Transactions as the Transactions store holds them, newest first, since a store lists
    /// them in no particular order: the Ledger keeps no read copy (#1028).
    fn transactions_of(store: &LedgerStore) -> Vec<lib_transactions::Transaction> {
        let mut transactions = store.transactions_store().borrow().all_transactions();
        transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
        transactions
    }

    /// The Accounts as the Accounts ports hold them: the Ledger keeps no read copy (#1054).
    fn accounts_of(store: &LedgerStore) -> Vec<lib_accounts::Account> {
        lib_accounts::AccountsPorts::accounts(&*store.accounts_ports().borrow()).accounts()
    }

    /// The Bill Plans as the Bills ports hold them: the Ledger keeps no read copy (#1028).
    fn plans_of(store: &LedgerStore) -> Vec<lib_bills::BillPlan> {
        store.bills.borrow().bills().bill_plans()
    }

    /// The Bill Schedule as the Bills ports hold it, by due date and then Plan.
    fn entries_of(store: &LedgerStore) -> Vec<lib_bills::BillScheduleEntry> {
        let mut entries = store.bills.borrow().bills().entries();
        entries.sort_by_key(|entry| (entry.due, entry.plan_id));
        entries
    }

    /// Every Split as the Bills ports read them from the Transactions store.
    fn splits_of(store: &LedgerStore) -> Vec<lib_bills::BillSplit> {
        store
            .bills
            .borrow()
            .transactions()
            .splits_between(chrono::NaiveDate::MIN, chrono::NaiveDate::MAX)
    }

    /// The Budgets as the Budgets store holds them: the Ledger keeps no read copy (#1028).
    fn budgets_of(store: &LedgerStore) -> Vec<lib_budgets::Budget> {
        store.budgets_store().borrow().budgets().budgets()
    }

    /// Commits what was written, as the input's `send` would, then opens the Ledger in `dir`
    /// again a day later, so a re-seed would show as shifted dates.
    fn restart(
        cx: &mut TestAppContext,
        (store, pending): (Entity<LedgerStore>, PendingWrites),
        writer: lib_database::Writer,
        dir: &std::path::Path,
        today: chrono::NaiveDate,
    ) -> Entity<LedgerStore> {
        pending.commit();
        crate::quit(&writer);
        drop(store);
        drop(writer);
        on_storage(cx, storage_in(dir, today.succ_opt().expect("a date"))).0
    }

    #[gpui::test]
    fn paying_matching_skipping_and_editing_a_plan_survive_a_restart(cx: &mut TestAppContext) {
        use lib_bill_payments::interactor::PayBill;
        use lib_bills::interactor::{EditBillPlan, MatchBill, SkipBill};
        use lib_bills::{BillPlanDraft, Resolution};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let OnStorage {
            store,
            bills,
            payments,
            pending,
            ..
        } = on_storage_with_bills(cx, storage);
        let (plans, entries) = bills_of(cx, &store);
        let bill_accounts = bills.borrow().accounts().bill_accounts();
        let bill_categories = bills.borrow().categories().bill_categories();
        let (matched, split) = store.read_with(cx, |store, _| {
            entries
                .iter()
                .filter(|entry| entry.is_open())
                .find_map(|entry| {
                    lib_bills::match_candidates(
                        &plans_of(store),
                        &entries_of(store),
                        &splits_of(store),
                        &bill_accounts,
                        &bill_categories,
                        entry.id(),
                    )
                    .first()
                    .map(|split| (entry.id(), *split))
                })
                .expect("the stub leaves an entry with a Match candidate")
        });
        let mut open = entries
            .iter()
            .filter(|entry| entry.is_open() && entry.id() != matched)
            .map(lib_bills::BillScheduleEntry::id);
        let paid = open.next().expect("a second open entry");
        let skipped = open
            .find(|entry| entry.plan_id != paid.plan_id)
            .expect("a third open entry");
        let plan = plans[0].clone();

        // Pay is the Bill Payments module's, through its port; Match, Skip and the edit are the
        // Bills module's, through its Interactors. One commit lands them all.
        PayBill {
            entry: paid,
            amount: "42.50".parse().expect("a decimal"),
            date: today,
        }
        .run(&mut *payments.borrow_mut())
        .expect("the entry is paid");
        {
            let mut ports = bills.borrow_mut();
            let ports = ports.split_mut();
            MatchBill {
                entry: matched,
                split,
            }
            .run(
                ports.bills,
                ports.transactions,
                ports.accounts,
                ports.categories,
            )
            .expect("the entry is matched");
            SkipBill { entry: skipped }
                .run(ports.bills)
                .expect("the entry is skipped");
            EditBillPlan {
                id: plan.id,
                draft: BillPlanDraft {
                    name: "Renamed plan".into(),
                    category_id: plan.category_id,
                    unit: plan.unit,
                    account_id: plan.account_id,
                    payee_id: plan.payee_id,
                    planned_amount: plan.planned_amount.clone(),
                    amount_kind: plan.amount_kind,
                    recurrence: plan.recurrence,
                    first_due: plan.first_due,
                    ends_on: plan.ends_on,
                    attention_lead: plan.attention_lead,
                },
                is_active: plan.is_active,
                today,
            }
            .run(ports.bills, ports.accounts, ports.categories)
            .expect("the plan is edited");
        }
        cx.update(|cx| crate::bill_payments::announce(&store, cx));
        let before = bills_of(cx, &store);
        let transactions = store.read_with(cx, |store, _| transactions_of(store));

        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        assert_eq!(bills_of(cx, &reopened), before);
        let after = &before.1;
        let resolution = |id| lib_bills::entry(after, id).map(|entry| entry.resolution.clone());
        assert_eq!(resolution(matched), Some(Resolution::Paid(split)));
        assert!(matches!(
            resolution(skipped),
            Some(Resolution::Skipped { .. })
        ));
        let Some(Resolution::Paid(payment)) = resolution(paid) else {
            panic!("the paid entry stays Paid");
        };
        assert!(
            reopened.read_with(cx, |store, _| transactions_of(store)
                .iter()
                .any(|t| t.id == payment.transaction_id)),
            "Pay's Transaction is stored with it"
        );
        assert_eq!(
            reopened.read_with(cx, |store, _| transactions_of(store)),
            transactions
        );
        assert_eq!(before.0[0].name, "Renamed plan");
    }

    #[gpui::test]
    fn deleting_an_account_leaves_the_entries_its_transactions_paid_unmatched_after_a_restart(
        cx: &mut TestAppContext,
    ) {
        use lib_bills::Resolution;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let (entry, account) = on.store.read_with(cx, |store, _| {
            entries_of(store)
                .iter()
                .find_map(|entry| match entry.resolution {
                    Resolution::Paid(split) => transactions_of(store)
                        .iter()
                        .find(|t| t.id == split.transaction_id)
                        .map(|t| (entry.id(), t.account_id)),
                    _ => None,
                })
                .expect("the stub pays an entry")
        });
        let resolution = |cx: &mut TestAppContext, store: &Entity<LedgerStore>| {
            store.read_with(cx, |store, _| {
                lib_bills::entry(&entries_of(store), entry).map(|entry| entry.resolution.clone())
            })
        };

        // An Account a Bill Plan pays from can't be deleted, so its Plans move to another Account
        // first: what's tested is the Unmatch.
        let other = on.store.read_with(cx, |store, _| {
            accounts_of(store)
                .iter()
                .find(|a| a.id != account)
                .expect("a second Account")
                .id
        });
        {
            let mut ports = on.bills.borrow_mut();
            let plans = ports.bills().bill_plans();
            for mut plan in plans.into_iter().filter(|plan| plan.account_id == account) {
                plan.account_id = other;
                ports.split_mut().bills.upsert_bill_plan(plan);
            }
        }
        cx.update(|cx| crate::bills::ports::announce(&on.store, cx));
        on.pending.commit();
        let budgets = on.store.read_with(cx, |store, _| budgets_of(store));
        assert!(
            budgets.iter().any(|b| b.account_ids.contains(&account)),
            "a stub Budget counts the Account"
        );

        delete_account(cx, &on, account).expect("no Plan pays from it now");
        assert_eq!(resolution(cx, &on.store), Some(Resolution::Unresolved));
        on.store.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().all(|a| a.id != account));
            assert!(
                transactions_of(store)
                    .iter()
                    .all(|t| t.account_id != account)
            );
            assert_eq!(
                budgets_of(store),
                budgets,
                "the Budgets store keeps the id in memory until it reloads; a write skips it"
            );
        });

        let OnStorage { store, pending, .. } = on;
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);

        assert_eq!(resolution(cx, &reopened), Some(Resolution::Unresolved));
        reopened.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().all(|a| a.id != account));
            assert!(
                budgets_of(store)
                    .iter()
                    .all(|b| !b.account_ids.contains(&account)),
                "the file dropped the Account from every Budget"
            );
        });
    }

    /// An Account a Bill Plan pays from is refused by `lib-account-deletion` before anything is
    /// written, so a restart finds the Account and its Transactions as they were.
    #[gpui::test]
    fn deleting_an_account_a_bill_plan_pays_from_is_refused_whole(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date");
        let storage = storage_in(dir.path(), today);
        let writer = storage.writer.clone();
        let on = on_storage_with_bills(cx, storage);
        let (account, plans, booked) = on.store.read_with(cx, |store, _| {
            let account = plans_of(store)[0].account_id;
            let plans = plans_of(store)
                .iter()
                .filter(|plan| plan.account_id == account)
                .count();
            let booked = transactions_of(store)
                .iter()
                .filter(|t| t.account_id == account)
                .count();
            (account, plans, booked)
        });
        assert!(booked > 0, "the stub books to a Bill Plan's Account");

        assert_eq!(
            delete_account(cx, &on, account),
            Err(lib_account_deletion::Error::PaidFromByBillPlans { plans })
        );

        let OnStorage { store, pending, .. } = on;
        let reopened = restart(cx, (store, pending), writer, dir.path(), today);
        reopened.read_with(cx, |store, _| {
            assert!(accounts_of(store).iter().any(|a| a.id == account));
            assert_eq!(
                transactions_of(store)
                    .iter()
                    .filter(|t| t.account_id == account)
                    .count(),
                booked
            );
        });
    }
}
