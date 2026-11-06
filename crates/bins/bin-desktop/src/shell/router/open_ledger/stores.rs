//! The stores an Open Ledger's modules share, and the stored reads built over them (#1057): what
//! [`super::OpenLedger::build`] assembles once from the `Stores` it is given and hands each page,
//! so no page reaches through an Entity for its data. The Open Ledger keeps them for what runs
//! after it is built: a failed write's reload (`write_failures.rs`), a new month's Bill Schedule
//! and the `*_for_test` hooks `Root`'s test support calls.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use gpui::{App, Entity};
use lib_database::{
    PayeesStore, SharedBudgetsStore, SharedCategoriesStore, SharedDocumentsStore,
    SharedInstitutionsStore, SharedInventoryStore, SharedTagsStore, SharedTransactionsStore,
    SharedUnitsStore, StoredAccountReads, StoredCategoryRows, StoredDocumentReads,
    StoredInventoryReads, Stores,
};
use lib_institutions::InstitutionsPorts as _;

use crate::accounts::ports::DesktopPorts as DesktopAccountsPorts;
use crate::announcer::{Announcer, Domain};
use crate::bills::ports::DesktopPorts as DesktopBillsPorts;
use crate::categories::ports::{CategoriesWrites, CategoryStores};
use crate::institutions::ports::DesktopPorts as DesktopInstitutionsPorts;
use crate::payees::ports::DesktopPorts as DesktopPayeesPorts;

/// The shared stores and ports a Ledger's modules and the cross-domain modules write, each one
/// value however many hold it.
pub struct LedgerStores {
    pub institutions: DesktopInstitutionsPorts,
    /// The Institutions store under the Institutions ports, which the Accounts' reads hold too.
    pub institutions_store: SharedInstitutionsStore,
    pub accounts: DesktopAccountsPorts,
    /// The usage of the Accounts and the Units and Institutions they may name, over the stores
    /// (#993, #1052, #1053): the Accounts ports' reads and Account delete's.
    pub account_reads: StoredAccountReads,
    pub categories: SharedCategoriesStore,
    /// The writes made to the Categories store, which the Categories announce from.
    pub categories_writes: CategoriesWrites,
    pub tags: SharedTagsStore,
    pub payees: DesktopPayeesPorts,
    pub transactions: SharedTransactionsStore,
    pub bills: DesktopBillsPorts,
    pub budgets: SharedBudgetsStore,
    pub inventory: SharedInventoryStore,
    pub documents: SharedDocumentsStore,
    /// The Units store the Units ports share and the Accounts, Institutions, Bills, Tags and
    /// Inventory reads take Units from: the Ledger file's, or the Sample Ledger's in memory in a
    /// test.
    pub units: SharedUnitsStore,
}

/// [`LedgerStores::build`]'s result: the shared stores, the Units ports over them, which the Units
/// Setup and the Open Ledger hold typed, and the commit and failure channel `stores` came with.
pub struct BuiltStores {
    pub stores: LedgerStores,
    pub units: crate::units::DesktopPorts,
    pub commit: Rc<dyn lib_database::Commit>,
    pub failures: mpsc::Receiver<lib_database::WriteFailure>,
}

impl LedgerStores {
    /// The shared stores and ports over `stores`, wired once: the one place a Ledger's stores are
    /// shared out, for [`super::OpenLedger::build`] and for a test that builds a module on
    /// `Stores::fake(FakeStores::sample(today))`.
    pub fn build(stores: Stores) -> BuiltStores {
        let Stores {
            units,
            institutions,
            accounts,
            categories,
            tags,
            payees,
            transactions,
            bills,
            budgets,
            inventory,
            documents,
            commit,
            failures,
        } = stores;
        // The Transactions and Bills stores are shared: Pay it directly writes both (#779), and
        // Payees counts what names a Payee in them (#992). The Payees ports are shared by the
        // Payees module, the cross-domain adapters, the Documents and the Bills reads
        // (#831, #832). The Categories store is shared by the Categories module, Category delete,
        // the Bills and Payees reads (#858, #859, #1005).
        let (categories, categories_writes) =
            crate::categories::ports::counted(Box::new(categories));
        let transactions: SharedTransactionsStore = Rc::new(RefCell::new(transactions));
        let bills: lib_database::SharedBillsStore = Rc::new(RefCell::new(bills));
        let payees = crate::payees::LedgerBackedPorts::shared(
            payees,
            lib_database::StoredPayeeReads {
                transactions: transactions.clone(),
                bills: bills.clone(),
                categories: categories.clone(),
            },
        );
        // The Tags store is shared by the Tags page and Tag remove and merge (#862, #1003).
        let tags: SharedTagsStore = Rc::new(RefCell::new(tags));
        // The Inventory store is shared by the Inventory page and the Documents reads (#866, #867).
        let inventory: SharedInventoryStore = Rc::new(RefCell::new(inventory));
        // The Budgets store is shared by the Budgets module, Category delete, the Accounts, Units
        // and Categories usage counts and the Categories Monthly budget (#952).
        let budgets: SharedBudgetsStore = Rc::new(RefCell::new(budgets));
        // The Units store is shared by the Units ports and the Accounts' and Institutions' reads of
        // the Units they may name (#1052). The Units ports count what names a Unit in the Accounts
        // and Institutions ports, so those read the store itself, not the Units ports, or each
        // would hold the other.
        let units: SharedUnitsStore = Rc::new(RefCell::new(units));
        // The Institutions store is shared by the Institutions ports and the Accounts' reads of the
        // Institutions an Account may be at (#1053), for the same reason as the Units store: the
        // Institutions ports read the Accounts ports, so owning it there would be a cycle.
        let institutions: SharedInstitutionsStore = Rc::new(RefCell::new(institutions));
        // The Accounts store is shared by the Accounts module, the Units, Bills and Documents reads
        // (#853), and the Institutions' reads of which Accounts name one (#876).
        let account_reads = StoredAccountReads {
            transactions: transactions.clone(),
            budgets: budgets.clone(),
            units: units.clone(),
            institutions: institutions.clone(),
        };
        let accounts = crate::accounts::LedgerBackedPorts::shared(accounts, account_reads.clone());
        let institutions_store = institutions.clone();
        let institutions = crate::institutions::LedgerBackedPorts::shared(
            institutions,
            accounts.clone(),
            units.clone(),
        );
        // The Units ports count what names each Unit in the Accounts, Transactions, Bills,
        // Budgets, Inventory and Institutions stores, and answer the Bills reads of the Unit codes
        // (#993), so they are built before the Bills ports.
        let units = crate::units::LedgerBackedPorts::shared(
            units,
            lib_database::StoredUnitReads {
                accounts: accounts.clone(),
                transactions: transactions.clone(),
                bills: bills.clone(),
                budgets: budgets.clone(),
                inventory: inventory.clone(),
                institutions: institutions.clone(),
            },
        );
        let bills = crate::bills::LedgerBackedPorts::shared(
            bills,
            payees.clone(),
            accounts.clone(),
            categories.clone(),
            transactions.clone(),
            units.clone(),
        );
        // The Documents store is shared by the Documents page, the Document Types page and the
        // Inventory's Documents reads (#963, #966, #1014).
        let documents: SharedDocumentsStore = Rc::new(RefCell::new(documents));
        BuiltStores {
            stores: Self {
                institutions,
                institutions_store,
                accounts,
                account_reads,
                categories,
                categories_writes,
                tags,
                payees,
                transactions,
                bills,
                budgets,
                inventory,
                documents,
                units: units.clone(),
            },
            units,
            commit,
            failures,
        }
    }

    /// The other domains' stores as `lib_inventory`'s reads, which the Inventory page and its
    /// Dialogs hold (#1014).
    pub fn inventory_reads(&self) -> StoredInventoryReads {
        StoredInventoryReads {
            units: self.units.clone(),
            inventory: self.inventory.clone(),
            documents: self.documents.clone(),
        }
    }

    /// The other domains' stores as `lib_documents`' reads, which the Documents page's Presenter
    /// holds (#963).
    pub fn document_reads(&self) -> StoredDocumentReads {
        StoredDocumentReads {
            transactions: self.transactions.clone(),
            payees: self.payees.clone(),
            accounts: self.accounts.clone(),
            institutions: self.institutions.clone(),
            bills: self.bills.borrow().store(),
            inventory: self.inventory.clone(),
        }
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

    /// What the Categories ports read and write beyond the Categories themselves: the Budgets
    /// store and the rows naming a Category.
    pub fn category_stores(&self) -> CategoryStores {
        CategoryStores {
            categories: self.categories.clone(),
            writes: self.categories_writes.clone(),
            budgets: self.budgets.clone(),
            rows: self.category_rows(),
        }
    }

    /// Adds a Location priced in `unit` to the Inventory store, so a test can delete a Unit one
    /// names (#885).
    #[doc(hidden)]
    pub fn add_location_in_for_test(
        &self,
        unit: lib_core::UnitId,
        announcer: &Entity<Announcer>,
        cx: &mut App,
    ) {
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
        announcer.update(cx, |announcer, cx| {
            announcer.announce(Domain::Units, cx);
            announcer.announce(Domain::Inventory, cx);
        });
    }

    /// Removes every Location from the Inventory store through the Interactor, as the Remove
    /// location Dialog does.
    #[doc(hidden)]
    pub fn empty_inventory_for_test(&self, announcer: &Entity<Announcer>, cx: &mut App) {
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
        announcer.update(cx, |announcer, cx| {
            announcer.announce(Domain::Inventory, cx)
        });
    }

    /// Writes an empty default Budget in `unit` straight to the Budgets store, which then names
    /// the Unit.
    #[doc(hidden)]
    pub fn add_budget_in_for_test(
        &self,
        unit: lib_core::UnitId,
        announcer: &Entity<Announcer>,
        cx: &mut App,
    ) {
        let budget = lib_budgets::personal_spending(unit, &[]);
        self.budgets
            .borrow_mut()
            .budgets_mut()
            .upsert_budget(&budget);
        announcer.update(cx, |announcer, cx| {
            announcer.announce(Domain::Units, cx);
            announcer.announce(Domain::Budgets, cx);
        });
    }

    /// Writes a Payee called `name` straight to the Payees store, so a test can meet the refusal
    /// of an Import that creates the same name.
    #[doc(hidden)]
    pub fn add_payee_for_test(&self, name: &str, announcer: &Entity<Announcer>, cx: &mut App) {
        PayeesStore::payees_mut(&mut *self.payees.borrow_mut()).upsert_payee(lib_payees::Payee {
            id: lib_core::PayeeId::new(),
            name: name.to_owned(),
            aliases: Vec::new(),
            default_category: None,
            is_active: true,
        });
        announcer.update(cx, |announcer, cx| announcer.announce(Domain::Payees, cx));
    }

    /// Gives Institution `id` Unit `unit` as its Usual unit, so a test can delete a Unit an
    /// Institution names.
    #[doc(hidden)]
    pub fn name_usual_unit_for_test(
        &self,
        id: lib_core::InstitutionId,
        unit: lib_core::UnitId,
        announcer: &Entity<Announcer>,
        cx: &mut App,
    ) {
        {
            let mut ports = self.institutions.borrow_mut();
            let Some(mut institution) = ports.institutions().institution(id) else {
                return;
            };
            institution.usual_unit = Some(unit);
            ports.institutions_mut().upsert_institution(institution);
        }
        announcer.update(cx, |announcer, cx| {
            announcer.announce(Domain::Institutions, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_database::{Commit, PendingWrites, StoredPayeeReads};

    use super::*;
    use lib_bills::BillsPorts as _;

    /// The Desktop's start-up storage on the Ledger file in `dir`, the Sample Ledger as of `today`
    /// when there is none yet.
    fn storage_in(dir: &std::path::Path, today: chrono::NaiveDate) -> lib_database::Storage {
        let path = dir.join("ledger.pldb");
        if !path.exists() {
            lib_database::seed::sample_ledger(&path, today).expect("the Sample Ledger is created");
        }
        lib_database::open_storage(&path).expect("the Ledger opens")
    }

    /// The stores on `storage`, as an Entity a test reads them through, with the pending writes
    /// the test commits, as an input's `send` would.
    fn on_storage(
        cx: &mut TestAppContext,
        storage: lib_database::Storage,
    ) -> (Entity<LedgerStores>, PendingWrites) {
        let on = on_storage_with_bills(cx, storage);
        (on.store, on.pending)
    }

    /// The stores on the Ledger file, with the ports the modules that write beside them hold.
    struct OnStorage {
        store: Entity<LedgerStores>,
        /// The Accounts ports the stores share, which Account delete announces from.
        accounts: DesktopAccountsPorts,
        /// What the modules announce their writes through.
        announcer: Entity<Announcer>,
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
        let account_reads = StoredAccountReads {
            transactions: transactions.clone(),
            budgets: budgets.clone(),
            units: units.clone(),
            institutions: institutions_store.clone(),
        };
        let accounts = crate::accounts::LedgerBackedPorts::shared(
            Box::new(storage.accounts),
            account_reads.clone(),
        );
        let institutions = crate::institutions::LedgerBackedPorts::shared(
            institutions_store.clone(),
            accounts.clone(),
            units.clone(),
        );
        let bills = crate::bills::ports::LedgerBackedPorts::shared(
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
        let store = cx.new(|_| LedgerStores {
            institutions,
            institutions_store,
            accounts: accounts.clone(),
            account_reads,
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
        });
        OnStorage {
            store,
            accounts,
            announcer: cx.new(|_| Announcer::default()),
            bills,
            payments,
            deletion,
            category_deletion,
            pending,
        }
    }

    /// Deletes Account `id` through the Account Deletion module's port, then announces it,
    /// as its Dialog's confirm does.
    fn delete_account(
        cx: &mut TestAppContext,
        on: &OnStorage,
        id: lib_core::AccountId,
    ) -> lib_account_deletion::Result<lib_account_deletion::AccountDeleted> {
        let deleted = lib_account_deletion::interactor::DeleteAccount { id }
            .run(&mut *on.deletion.borrow_mut());
        cx.update(|cx| crate::account_deletion::refresh(&on.accounts, &on.announcer, cx));
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
        let wallet = lib_accounts::sample_accounts()
            .into_iter()
            .find(|account| account.name == "Wallet")
            .expect("a sample Account")
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
                .expect("a sample Category with Splits, a Bill Plan and a Payee default")
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

    /// The Categories as the Categories store holds them.
    fn categories_of(store: &LedgerStores) -> Vec<lib_categories::Category> {
        store.categories.borrow().categories().categories()
    }

    /// The Payees as the Payees store holds them.
    fn payees_of(store: &LedgerStores) -> Vec<lib_payees::Payee> {
        PayeesStore::payees(&*store.payees.borrow()).payees()
    }

    /// The Bill Plans and the Bill Schedule as the Bills ports hold them, the entries by due date
    /// and then Plan, since a port lists them in no particular order.
    fn bills_of(
        cx: &mut TestAppContext,
        store: &Entity<LedgerStores>,
    ) -> (Vec<lib_bills::BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
        store.read_with(cx, |store, _| (plans_of(store), entries_of(store)))
    }

    /// The Transactions as the Transactions store holds them, newest first, since a store lists
    /// them in no particular order.
    fn transactions_of(store: &LedgerStores) -> Vec<lib_transactions::Transaction> {
        let mut transactions = store.transactions.borrow().all_transactions();
        transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
        transactions
    }

    /// The Accounts as the Accounts ports hold them.
    fn accounts_of(store: &LedgerStores) -> Vec<lib_accounts::Account> {
        lib_accounts::AccountsPorts::accounts(&*store.accounts.borrow()).accounts()
    }

    /// The Bill Plans as the Bills ports hold them.
    fn plans_of(store: &LedgerStores) -> Vec<lib_bills::BillPlan> {
        store.bills.borrow().bills().bill_plans()
    }

    /// The Bill Schedule as the Bills ports hold it, by due date and then Plan.
    fn entries_of(store: &LedgerStores) -> Vec<lib_bills::BillScheduleEntry> {
        let mut entries = store.bills.borrow().bills().entries();
        entries.sort_by_key(|entry| (entry.due, entry.plan_id));
        entries
    }

    /// Every Split as the Bills ports read them from the Transactions store.
    fn splits_of(store: &LedgerStores) -> Vec<lib_bills::BillSplit> {
        store
            .bills
            .borrow()
            .transactions()
            .splits_between(chrono::NaiveDate::MIN, chrono::NaiveDate::MAX)
    }

    /// The Budgets as the Budgets store holds them.
    fn budgets_of(store: &LedgerStores) -> Vec<lib_budgets::Budget> {
        store.budgets.borrow().budgets().budgets()
    }

    /// Commits what was written, as the input's `send` would, then opens the Ledger in `dir`
    /// again a day later, so a re-seed would show as shifted dates.
    fn restart(
        cx: &mut TestAppContext,
        (store, pending): (Entity<LedgerStores>, PendingWrites),
        writer: lib_database::Writer,
        dir: &std::path::Path,
        today: chrono::NaiveDate,
    ) -> Entity<LedgerStores> {
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
            announcer,
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
                .expect("the sample leaves an entry with a Match candidate")
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
        cx.update(|cx| crate::bill_payments::announce(&announcer, cx));
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
                .expect("the sample pays an entry")
        });
        let resolution = |cx: &mut TestAppContext, store: &Entity<LedgerStores>| {
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
        cx.update(|cx| crate::bills::ports::announce(&on.announcer, cx));
        on.pending.commit();
        let budgets = on.store.read_with(cx, |store, _| budgets_of(store));
        assert!(
            budgets.iter().any(|b| b.account_ids.contains(&account)),
            "a sample Budget counts the Account"
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
        assert!(booked > 0, "the sample books to a Bill Plan's Account");

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
