//! A Client's Ledger file, opened at start-up: the shared writer on the `[Personal-Ledger] file`,
//! the Client migrations, then the Units, Institutions, Accounts, Categories, Tags, Payees,
//! Transactions, Bills, Budgets, Inventory and Documents adapters on the pending batch every
//! adapter shares. The Desktop and the TUI both open it here,
//! so they read and write the same rows (ADR-0046), and both build their Shell on the [`Stores`]
//! made from it.

use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;

use crate::{
    AccountsAdapter, BillsAdapter, BudgetsAdapter, CategoriesAdapter, DocumentsAdapter, Error,
    InstitutionsAdapter, InventoryAdapter, PayeesAdapter, PendingWrites, Result, TagsAdapter,
    TransactionsAdapter, UnitsAdapter, WriteFailure, Writer, adapter::store,
};

/// The Ledger file as start-up opens it: each adapter, the pending batch the bin commits after
/// each Interactor call, and the writer every adapter saves through, with its failure reports.
pub struct Storage {
    pub units: UnitsAdapter,
    pub institutions: InstitutionsAdapter,
    pub accounts: AccountsAdapter,
    pub categories: CategoriesAdapter,
    pub tags: TagsAdapter,
    pub payees: PayeesAdapter,
    pub transactions: TransactionsAdapter,
    pub bills: BillsAdapter,
    pub budgets: BudgetsAdapter,
    pub inventory: InventoryAdapter,
    pub documents: DocumentsAdapter,
    pub pending: PendingWrites,
    /// Flushed when the Client quits, so the last writes land before the thread dies.
    pub writer: Writer,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// What a Client's Shell is built on: each module's store, the one commit that ends every input's
/// writes, every store's at once, and the channel a failed commit is reported on. Made from the
/// [`Storage`] start-up opened, or from in-memory stores for a test ([`Stores::fake`]), so
/// building the Shell takes one argument for storage. The writer isn't here: the Client keeps it
/// from the [`Storage`] to flush as it quits.
///
/// Each store is [`Recorded`](store::Recorded): it notes its domain in the commit's
/// [`Written`](crate::Written) on every mutable access, so the [`Committed`](crate::Committed) the
/// commit returns names every domain the input wrote.
pub struct Stores {
    pub units: store::Recorded<dyn store::UnitsStore>,
    pub institutions: store::Recorded<dyn store::InstitutionsStore>,
    pub accounts: store::Recorded<dyn store::AccountsStore>,
    pub categories: store::Recorded<dyn store::CategoriesStore>,
    pub tags: store::Recorded<dyn store::TagsStore>,
    pub payees: store::Recorded<dyn store::PayeesStore>,
    pub transactions: store::Recorded<dyn store::TransactionsStore>,
    pub bills: store::Recorded<dyn store::BillsStore>,
    pub budgets: store::Recorded<dyn store::BudgetsStore>,
    pub inventory: store::Recorded<dyn store::InventoryStore>,
    pub documents: store::Recorded<dyn store::DocumentsStore>,
    pub commit: Rc<dyn crate::Commit>,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// [`Stores`] with each store shared out once ([`Stores::share`]): what [`crate::LedgerPorts::over`]
/// builds on, so a Client can keep a clone of the [`SharedStores`](store::SharedStores) first.
pub struct SharedLedger {
    pub stores: store::SharedStores,
    pub commit: Rc<dyn crate::Commit>,
    pub failures: mpsc::Receiver<WriteFailure>,
}

impl Stores {
    /// Shares out each store once, keeping the commit and the failure channel beside them.
    pub fn share(self) -> SharedLedger {
        SharedLedger {
            stores: store::SharedStores {
                units: Rc::new(std::cell::RefCell::new(self.units)),
                institutions: Rc::new(std::cell::RefCell::new(self.institutions)),
                accounts: Rc::new(std::cell::RefCell::new(self.accounts)),
                categories: Rc::new(std::cell::RefCell::new(self.categories)),
                tags: Rc::new(std::cell::RefCell::new(self.tags)),
                payees: Rc::new(std::cell::RefCell::new(self.payees)),
                transactions: Rc::new(std::cell::RefCell::new(self.transactions)),
                bills: Rc::new(std::cell::RefCell::new(self.bills)),
                budgets: Rc::new(std::cell::RefCell::new(self.budgets)),
                inventory: Rc::new(std::cell::RefCell::new(self.inventory)),
                documents: Rc::new(std::cell::RefCell::new(self.documents)),
            },
            commit: self.commit,
            failures: self.failures,
        }
    }
}

/// The stores [`Stores::new`] wraps, unrecorded: the adapters or the in-memory fakes.
pub struct StoreParts {
    pub units: Box<dyn store::UnitsStore>,
    pub institutions: Box<dyn store::InstitutionsStore>,
    pub accounts: Box<dyn store::AccountsStore>,
    pub categories: Box<dyn store::CategoriesStore>,
    pub tags: Box<dyn store::TagsStore>,
    pub payees: Box<dyn store::PayeesStore>,
    pub transactions: Box<dyn store::TransactionsStore>,
    pub bills: Box<dyn store::BillsStore>,
    pub budgets: Box<dyn store::BudgetsStore>,
    pub inventory: Box<dyn store::InventoryStore>,
    pub documents: Box<dyn store::DocumentsStore>,
}

impl Stores {
    /// `parts`, each wrapped once in a recorder noting its domain in `written`, which `commit`
    /// drains, so the commit says what each input wrote.
    pub fn new(
        parts: StoreParts,
        written: &crate::Written,
        commit: Rc<dyn crate::Commit>,
        failures: mpsc::Receiver<WriteFailure>,
    ) -> Self {
        use crate::Domain;
        Self {
            units: store::Recorded::new(Domain::Units, written.clone(), parts.units),
            institutions: store::Recorded::new(
                Domain::Institutions,
                written.clone(),
                parts.institutions,
            ),
            accounts: store::Recorded::new(Domain::Accounts, written.clone(), parts.accounts),
            categories: store::Recorded::new(Domain::Categories, written.clone(), parts.categories),
            tags: store::Recorded::new(Domain::Tags, written.clone(), parts.tags),
            payees: store::Recorded::new(Domain::Payees, written.clone(), parts.payees),
            transactions: store::Recorded::new(
                Domain::Transactions,
                written.clone(),
                parts.transactions,
            ),
            bills: store::Recorded::new(Domain::Bills, written.clone(), parts.bills),
            budgets: store::Recorded::new(Domain::Budgets, written.clone(), parts.budgets),
            inventory: store::Recorded::new(Domain::Inventory, written.clone(), parts.inventory),
            documents: store::Recorded::new(Domain::Documents, written.clone(), parts.documents),
            commit,
            failures,
        }
    }
}

impl From<Storage> for Stores {
    fn from(storage: Storage) -> Self {
        let written = storage.pending.written().clone();
        Self::new(
            StoreParts {
                units: Box::new(storage.units),
                institutions: Box::new(storage.institutions),
                accounts: Box::new(storage.accounts),
                categories: Box::new(storage.categories),
                tags: Box::new(storage.tags),
                payees: Box::new(storage.payees),
                transactions: Box::new(storage.transactions),
                bills: Box::new(storage.bills),
                budgets: Box::new(storage.budgets),
                inventory: Box::new(storage.inventory),
                documents: Box::new(storage.documents),
            },
            &written,
            Rc::new(storage.pending),
            storage.failures,
        )
    }
}

/// Opens the Ledger at `path`, migrating it if need be, and loads every adapter, blocking.
///
/// Opening only opens and writes nothing (ADR-0054): a Ledger is created, with what every new one
/// starts with, by `lib-ledger-creation`'s [`LedgerCreationAdapter`](crate::LedgerCreationAdapter),
/// never here. A migrated file with no base Unit, no Budget or no Document Type is missing what
/// every new Ledger starts with, so it was cut short, made by hand or made before Budgets or
/// Documents persisted, and it is refused rather than completed. The Personal Institution isn't
/// checked, because a person may delete it. A path with no file is refused before anything is
/// made there.
///
/// # Errors
/// Returns an [`Error::Io`] of kind `NotFound` for a path with no file,
/// [`Error::IncompleteLedger`] for a file with no base Unit, Budget or Document Type, or the error
/// opening, migrating or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path) -> Result<Storage> {
    // Checked here rather than left to the Writer, whose open makes a missing file.
    if !path.try_exists()? {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no Ledger file at {}", path.display()),
        )));
    }
    let (writer, failures) = Writer::open_ledger(path)?;
    let pending = PendingWrites::new(writer.clone());
    let units = match UnitsAdapter::load(pending.clone()) {
        Err(Error::NotFound(_)) => return Err(Error::IncompleteLedger),
        loaded => loaded?,
    };
    if BudgetsAdapter::is_empty(&writer)? || DocumentsAdapter::has_no_types(&writer)? {
        return Err(Error::IncompleteLedger);
    }
    let institutions = InstitutionsAdapter::load(pending.clone())?;
    let accounts = AccountsAdapter::load(pending.clone())?;
    let categories = CategoriesAdapter::load(pending.clone())?;
    let tags = TagsAdapter::load(pending.clone())?;
    let payees = PayeesAdapter::load(pending.clone())?;
    let transactions = TransactionsAdapter::load(pending.clone())?;
    let bills = BillsAdapter::load(pending.clone())?;
    let budgets = BudgetsAdapter::load(pending.clone())?;
    let inventory = InventoryAdapter::load(pending.clone())?;
    let documents = DocumentsAdapter::load(pending.clone())?;
    Ok(Storage {
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
        pending,
        writer,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use lib_bills::{BillScheduleRepository, Resolution};
    use lib_transactions::TransactionRepository;

    use lib_institutions::InstitutionRepository as _;

    use chrono::NaiveDate;

    use super::*;
    use crate::{Commit, seed};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger created at `path` as of [`today`], opened.
    fn seeded(path: &Path) -> Storage {
        seed::sample_ledger(path, today()).expect("the Sample Ledger is created");
        open_storage(path).expect("a new Ledger opens")
    }

    #[test]
    fn a_new_ledger_opens_seeded_and_a_write_survives_a_restart() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");

        let mut storage = seeded(&path);
        let seeded = storage.transactions.transactions();
        assert_eq!(seeded, seed::sample_transactions(today()));
        let gone = seeded[0].id;
        storage.transactions.delete_transaction(gone);
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);

        let reopened = open_storage(&path).expect("the Ledger opens again");

        let mut expected = seeded;
        expected.remove(0);
        assert_eq!(reopened.transactions.transactions(), expected);
        assert!(reopened.transactions.transaction(gone).is_none());
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_budgets_and_an_edit_survives_a_restart() {
        use lib_budgets::BudgetRepository as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.budgets.budgets(), seed::sample_budgets(today()));
        let mut joint = storage
            .budgets
            .budget(lib_core::BudgetId::seed(2))
            .expect("a sample Budget");
        joint.name = "Shared house".into();
        storage.budgets.upsert_budget(&joint);
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);

        let reopened = open_storage(&path).expect("the Ledger opens again");
        let mut expected = seed::sample_budgets(today());
        expected[1] = joint;
        assert_eq!(reopened.budgets.budgets(), expected);
    }

    /// A Ledger made as `:new` makes one at `path`, then `sql` run on it and the file closed, as
    /// a hand edit or an older build would leave it.
    fn created_then(path: &Path, sql: &'static [&'static str]) {
        use lib_ledger_creation::LedgerCreation as _;

        crate::LedgerCreationAdapter::new()
            .create_ledger(path, &lib_ledger_creation::contract::usd_ledger())
            .expect("nothing is at the path");
        let (writer, _) = Writer::open_ledger(path).expect("the Ledger opens");
        writer
            .run(move |pool| async move {
                for statement in sql {
                    sqlx::query(statement).execute(&pool).await?;
                }
                Ok(())
            })
            .expect("the edit is written");
        writer.close().expect("the writer is running");
    }

    /// Every new Ledger starts with a Budget, so a file with none is refused rather than given
    /// one, and opening it changes nothing in it.
    #[test]
    fn a_migrated_file_with_no_budget_is_refused_as_incomplete_and_left_as_it_was() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        created_then(&path, &["DELETE FROM budgets"]);
        let before = std::fs::read(&path).expect("the file reads");

        assert_eq!(open_storage(&path).err(), Some(Error::IncompleteLedger));
        assert_eq!(std::fs::read(&path).expect("the file reads"), before);
    }

    /// Every new Ledger starts with the default Document Types, so a file with none is refused
    /// rather than given them, and opening it changes nothing in it.
    #[test]
    fn a_migrated_file_with_no_document_type_is_refused_as_incomplete_and_left_as_it_was() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        created_then(&path, &["DELETE FROM document_types"]);
        let before = std::fs::read(&path).expect("the file reads");

        assert_eq!(open_storage(&path).err(), Some(Error::IncompleteLedger));
        assert_eq!(std::fs::read(&path).expect("the file reads"), before);
    }

    /// A person may delete the Personal Institution, so a Ledger without it still opens.
    #[test]
    fn a_ledger_with_its_personal_institution_deleted_opens() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        created_then(&path, &["DELETE FROM institutions"]);

        let storage = open_storage(&path).expect("the Ledger opens");

        assert!(storage.institutions.institutions().is_empty());
    }

    /// A migrated file with no base Unit, as an interrupted create or an older build's cut-short
    /// first launch left it, is refused rather than completed, and nothing is written into it.
    #[test]
    fn a_migrated_file_with_no_base_unit_is_refused_as_incomplete_and_left_as_it_was() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let (writer, _) = Writer::open_ledger(&path).expect("the file is made and migrated");
        writer.close().expect("the writer is running");
        drop(writer);

        assert_eq!(open_storage(&path).err(), Some(Error::IncompleteLedger));
        assert_eq!(rows(&path), 0, "nothing was written into it");
    }

    /// Opening only opens (ADR-0054): a path with no file is refused, and no file is made there.
    #[test]
    fn a_missing_file_is_refused_and_not_made() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");

        let error = open_storage(&path).err().expect("there is no file to open");

        assert!(
            matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
            "refused as missing, got {error:?}"
        );
        assert!(!path.exists(), "no file was made");
    }

    /// A Ledger created as `:new` creates one opens with its starting rows.
    #[test]
    fn a_created_ledger_opens() {
        use lib_ledger_creation::LedgerCreation as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let ledger = lib_ledger_creation::contract::usd_ledger();
        crate::LedgerCreationAdapter::new()
            .create_ledger(&path, &ledger)
            .expect("nothing is at the path");

        let storage = open_storage(&path).expect("the Ledger opens");

        assert_eq!(storage.units.base_unit().base_unit(), ledger.base_unit.id);
        assert_eq!(storage.institutions.institutions(), [ledger.personal]);
    }

    /// Every row in the file's tables but the migrations', to show what a failed open left.
    fn rows(path: &Path) -> i64 {
        let (writer, _) = Writer::open_ledger(path).expect("the file opens");
        writer
            .run(|pool| async move {
                let tables: Vec<String> = sqlx::query_scalar(
                    "SELECT name FROM sqlite_master
                     WHERE type = 'table' AND name <> '_sqlx_migrations'
                       AND name NOT LIKE 'sqlite%'",
                )
                .fetch_all(&pool)
                .await?;
                let mut total = 0;
                for table in tables {
                    let count: i64 =
                        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM \"{table}\""))
                            .fetch_one(&pool)
                            .await?;
                    total += count;
                }
                Ok(total)
            })
            .expect("the rows are counted")
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_institutions_and_an_added_one_survives_a_restart() {
        use lib_institutions::{Institution, InstitutionRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.institutions.institutions(),
            seed::sample_institutions()
        );
        let bank = Institution {
            id: lib_core::InstitutionId::new(),
            name: "Bank X".into(),
            products: "savings".into(),
            usual_unit: None,
        };

        storage.institutions.upsert_institution(bank.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        let mut expected = seed::sample_institutions();
        expected.push(bank);
        assert_eq!(reopened.institutions.institutions(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_accounts_and_an_added_one_survives_a_restart() {
        use lib_accounts::Account;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.accounts.accounts().accounts(),
            seed::sample_accounts()
        );
        let savings = Account {
            id: lib_core::AccountId::new(),
            name: "Holiday Fund".into(),
            ..seed::sample_accounts()[1].clone()
        };

        storage
            .accounts
            .accounts_mut()
            .upsert_account(savings.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        let mut expected = seed::sample_accounts();
        expected.push(savings);
        assert_eq!(reopened.accounts.accounts().accounts(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_categories_and_an_added_one_survives_a_restart() {
        use lib_categories::{Category, CategoryRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.categories.categories(), seed::sample_categories());
        let gas = Category {
            id: lib_core::CategoryId::new(),
            name: "Gas".into(),
            parent: Some(lib_core::CategoryId::seed(3)),
            category_type: lib_core::CategoryTypes::Expense,
        };

        storage.categories.upsert_category(gas.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        assert_eq!(reopened.categories.category(gas.id), Some(gas));
        assert_eq!(
            reopened.categories.categories().len(),
            seed::sample_categories().len() + 1
        );
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_tags_and_an_added_one_survives_a_restart() {
        use lib_tags::Tag;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.tags.tags().tags(), seed::sample_tags());
        let holiday = Tag {
            id: lib_core::TagId::new(),
            name: "Holiday".into(),
            color: Some(lib_core::HexColor::from_rgb(0x4a, 0x7c, 0x9e)),
            is_active: true,
        };

        storage.tags.tags_mut().upsert_tag(holiday.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        assert_eq!(reopened.tags.tags().tag(holiday.id), Some(holiday));
        assert_eq!(
            reopened.tags.tags().tags().len(),
            seed::sample_tags().len() + 1
        );
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_inventory_and_an_added_location_survives_a_restart() {
        use lib_inventory::interactor::{AddArea, AddItem, AddLocation};
        use lib_inventory::{ItemDraft, LocationDraft};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.inventory.inventory().inventory(),
            seed::sample_inventory(today())
        );

        let port = storage.inventory.inventory_mut();
        let shack = AddLocation {
            draft: LocationDraft {
                name: "Beach shack".into(),
                ..LocationDraft::default()
            },
            unit: lib_core::UnitId::SEED_AUD,
        }
        .run(port)
        .expect("a new name");
        let deck = AddArea {
            location: shack,
            name: "Deck".into(),
        }
        .run(port)
        .expect("a new name");
        let kayak = AddItem {
            location: shack,
            area: deck,
            draft: ItemDraft {
                name: "Kayak".into(),
                replacement: lib_core::Money(900.into()),
            },
        }
        .run(port)
        .expect("the Area is the Location's");
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        assert!(storage.failures.try_recv().is_err(), "the writes landed");
        let written = storage.inventory.inventory().inventory();
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        let reread = reopened.inventory.inventory().inventory();
        assert_eq!(reread, written);
        assert_eq!(reread.item(kayak).map(|item| item.area), Some(deck));
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_payees_and_an_added_one_survives_a_restart() {
        use lib_payees::Payee;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.payees.payees().payees(), seed::sample_payees());
        let corner = Payee {
            id: lib_core::PayeeId::new(),
            name: "Corner Shop".into(),
            aliases: vec!["CORNER".into()],
            default_category: None,
            is_active: true,
        };

        storage.payees.payees_mut().upsert_payee(corner.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        assert_eq!(reopened.payees.payees().payee(corner.id), Some(corner));
    }

    #[test]
    fn a_new_ledger_opens_with_the_sample_bills_and_their_paid_entries() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let storage = seeded(&dir.path().join("ledger.pldb"));

        let expected = seed::sample_bills(today());
        assert_eq!(
            lib_bills::contract::snapshot(&storage.bills),
            (
                {
                    let mut plans = expected.plans.clone();
                    plans.sort_by_key(|plan| plan.id);
                    plans
                },
                {
                    let mut entries = expected.entries.clone();
                    entries.sort_by_key(lib_bills::BillScheduleEntry::id);
                    entries
                }
            )
        );
        assert!(
            storage
                .bills
                .entries()
                .iter()
                .any(|entry| matches!(entry.resolution, Resolution::Paid(_))),
            "the sample's Paid entries keep their Transactions"
        );
    }
}

/// What the commit says an input changed, read through [`Stores`] as a Client holds them: on the
/// in-memory fakes and on the Ledger file.
#[cfg(test)]
mod committed_tests {
    use std::rc::Rc;

    use chrono::NaiveDate;
    use lib_category_deletion::CategoryDeletion as _;
    use lib_documents::DocumentLink;

    use super::*;
    use crate::adapter::store::{
        DocumentsStore, PayeesStore, SharedStores, TagsStore, TransactionsStore as _,
    };
    use crate::{Committed, Domain, FakeStores, seed};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger in memory, as the UI tests build the Client on it.
    fn fakes() -> Stores {
        Stores::fake(FakeStores::sample(today())).0
    }

    /// The Sample Ledger created as a Ledger file in `dir`, opened as a Client opens it, with the
    /// writer to flush it through.
    fn file(dir: &Path) -> (Stores, Writer) {
        let path = dir.join("ledger.pldb");
        seed::sample_ledger(&path, today()).expect("the Sample Ledger is created");
        let storage = open_storage(&path).expect("the Ledger opens");
        let writer = storage.writer.clone();
        (Stores::from(storage), writer)
    }

    /// Every store shared, as a Client holds them through [`crate::LedgerPorts`], and the commit.
    fn share(stores: Stores) -> (SharedStores, Rc<dyn crate::Commit>) {
        let ledger = crate::LedgerPorts::new(
            stores,
            lib_core::Clock::fixed(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")),
        );
        (ledger.stores, ledger.commit)
    }

    /// Renames the first sample Tag, a write to the Tags alone.
    fn rename_a_tag(stores: &mut Stores) {
        let mut tag = stores.tags.tags().tags()[0].clone();
        tag.name.push_str(" (renamed)");
        stores.tags.tags_mut().upsert_tag(tag);
    }

    /// Writes the first Document Type back as it is, a write to the Document Types alone.
    fn rewrite_a_document_type(stores: &mut Stores) {
        let row = DocumentsStore::documents(&stores.documents).document_types()[0].clone();
        DocumentsStore::documents_mut(&mut stores.documents).upsert_document_type(&row);
    }

    fn wrote(domains: &[Domain]) -> Committed {
        Committed {
            wrote: domains.to_vec(),
            cascaded: Vec::new(),
        }
    }

    #[test]
    fn a_commit_with_no_writes_reports_nothing_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for stores in [fakes(), file(dir.path()).0] {
            let _ = stores.tags.tags().tags();

            assert!(stores.commit.commit().is_empty());
        }
    }

    #[test]
    fn a_write_puts_its_domain_in_wrote_once_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for mut stores in [fakes(), file(dir.path()).0] {
            rename_a_tag(&mut stores);
            rename_a_tag(&mut stores);

            assert_eq!(stores.commit.commit(), wrote(&[Domain::Tags]));
            assert!(stores.commit.commit().is_empty(), "the commit drains it");
        }
    }

    #[test]
    fn a_document_type_write_reports_document_types_alone_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for mut stores in [fakes(), file(dir.path()).0] {
            rewrite_a_document_type(&mut stores);

            assert_eq!(stores.commit.commit(), wrote(&[Domain::DocumentTypes]));
        }
    }

    #[test]
    fn a_tag_merge_reports_the_tags_and_the_transactions() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for stores in [fakes(), file(dir.path()).0] {
            let ledger = crate::LedgerPorts::new(
                stores,
                lib_core::Clock::fixed(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")),
            );
            let mut removal = ledger.tag_removal.borrow_mut();
            let tags = ledger.stores.tags.borrow().tags().tags();
            let source = tags
                .iter()
                .find(|tag| !removal.transactions_carrying(tag.id).is_empty())
                .expect("the sample tags a Transaction")
                .id;
            let target = tags
                .iter()
                .find(|tag| tag.id != source)
                .expect("the sample has two Tags")
                .id;

            lib_tag_removal::interactor::MergeTags { source, target }
                .run(&mut *removal)
                .expect("both Tags exist");

            assert_eq!(
                ledger.commit.commit(),
                wrote(&[Domain::Transactions, Domain::Tags])
            );
        }
    }

    #[test]
    fn a_category_delete_reports_every_domain_its_handover_writes() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for stores in [fakes(), file(dir.path()).0] {
            let (shared, commit) = share(stores);
            let mut deletion = crate::CategoryDeletionAdapter::new(
                shared.categories.clone(),
                shared.transactions.clone(),
                shared.bills.clone(),
                shared.payees.clone(),
                shared.budgets.clone(),
            );
            let categories = shared.categories.borrow().categories().categories();
            let id = categories
                .iter()
                .map(|category| category.id)
                .find(|&id| {
                    !deletion.transactions_using(id).is_empty()
                        && !deletion.bill_plans_using(id).is_empty()
                        && !deletion.payees_defaulting_to(id).is_empty()
                        && !deletion.budgets_limiting(id).is_empty()
                })
                .expect("a sample Category named by a Split, a Bill Plan, a Payee and a Limit");

            lib_category_deletion::interactor::DeleteCategory { id }
                .run(&mut deletion)
                .expect("a leaf Category deletes");

            let mut domains = commit.commit().wrote;
            domains.sort();
            assert_eq!(
                domains,
                [
                    Domain::Categories,
                    Domain::Payees,
                    Domain::Transactions,
                    Domain::Bills,
                    Domain::Budgets,
                ]
            );
        }
    }

    #[test]
    fn a_delete_that_cascades_on_the_file_reports_the_cascade() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (mut stores, _writer) = file(dir.path());
        let linked = DocumentsStore::documents(&stores.documents)
            .documents()
            .into_iter()
            .flat_map(|document| document.links)
            .find_map(|link| match link {
                DocumentLink::Transaction(id) => Some(id),
                _ => None,
            })
            .expect("a sample Document links a Transaction");

        stores
            .transactions
            .transactions_mut()
            .delete_transaction(linked);

        assert_eq!(
            stores.commit.commit(),
            Committed {
                wrote: vec![Domain::Transactions],
                cascaded: vec![Domain::Documents],
            }
        );
    }

    #[test]
    fn a_reload_by_domain_drops_only_that_domains_pending_writes() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (stores, writer) = file(dir.path());
        let (shared, commit) = share(stores);
        let mut tag = shared.tags.borrow().tags().tags()[0].clone();
        let stored_name = tag.name.clone();
        tag.name.push_str(" (dropped)");
        shared.tags.borrow_mut().tags_mut().upsert_tag(tag.clone());
        let mut payee = shared.payees.borrow().payees().payees()[0].clone();
        payee.name.push_str(" (kept)");
        shared
            .payees
            .borrow_mut()
            .payees_mut()
            .upsert_payee(payee.clone());

        shared.reload(Domain::Tags).expect("the Tags load");
        commit.commit();
        writer.flush().expect("the writer is running");

        let reopened = open_storage(&dir.path().join("ledger.pldb")).expect("the Ledger opens");
        assert_eq!(
            TagsStore::tags(&reopened.tags)
                .tag(tag.id)
                .map(|tag| tag.name),
            Some(stored_name.clone())
        );
        assert_eq!(
            shared.tags.borrow().tags().tag(tag.id).map(|tag| tag.name),
            Some(stored_name),
            "memory is read back too"
        );
        assert_eq!(
            PayeesStore::payees(&reopened.payees)
                .payee(payee.id)
                .map(|payee| payee.name),
            Some(payee.name)
        );
    }

    #[test]
    fn the_documents_and_the_document_types_reload_the_one_documents_store() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (stores, _writer) = file(dir.path());
        let (shared, _commit) = share(stores);
        let before = shared.documents.borrow().documents().document_types();
        let mut row = before[0].clone();
        row.name.push_str(" (dropped)");
        shared
            .documents
            .borrow_mut()
            .documents_mut()
            .upsert_document_type(&row);

        shared.reload(Domain::DocumentTypes).expect("the rows load");

        assert_eq!(
            shared.documents.borrow().documents().document_types(),
            before
        );
    }
}
