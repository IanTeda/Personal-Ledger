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
pub struct Stores {
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
    pub commit: Rc<dyn crate::Commit>,
    pub failures: mpsc::Receiver<WriteFailure>,
}

impl From<Storage> for Stores {
    fn from(storage: Storage) -> Self {
        Self {
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
            commit: Rc::new(storage.pending),
            failures: storage.failures,
        }
    }
}

/// Opens the Ledger at `path`, migrating it if need be, and loads every adapter, blocking.
///
/// Opening only opens (ADR-0054): a Ledger is created, with what every new one starts with, by
/// `lib-ledger-creation`'s [`LedgerCreationAdapter`](crate::LedgerCreationAdapter), never here. A
/// migrated file with no base Unit was cut short or made by hand, so it is refused rather than
/// completed. `open_storage` still makes and migrates a file missing at `path`, which it then
/// refuses the same way, so a Client checks the file is there first.
///
/// A Ledger with no Budget, one written before Budgets persisted, is given an empty default
/// Personal spending Budget in its base Unit on every Account in it that takes Transactions
/// (`lib_budgets::personal_spending`), so there is always an active Budget.
///
/// A Ledger with no Document Type, one written before Documents persisted, is given the nine
/// default types (ADR-0031) and no Documents, so there is always a Default type.
///
/// # Errors
/// Returns [`Error::IncompleteLedger`] for a file with no base Unit, or the error opening,
/// migrating or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path) -> Result<Storage> {
    let (writer, failures) = Writer::open_ledger(path)?;
    let pending = PendingWrites::new(writer.clone());
    let units = match UnitsAdapter::load(pending.clone()) {
        Err(Error::NotFound(_)) => return Err(Error::IncompleteLedger),
        loaded => loaded?,
    };
    let institutions = InstitutionsAdapter::load(pending.clone())?;
    let accounts = AccountsAdapter::load(pending.clone())?;
    let categories = CategoriesAdapter::load(pending.clone())?;
    let tags = TagsAdapter::load(pending.clone())?;
    let payees = PayeesAdapter::load(pending.clone())?;
    let transactions = TransactionsAdapter::load(pending.clone())?;
    let bills = BillsAdapter::load(pending.clone())?;
    if BudgetsAdapter::is_empty(&writer)? {
        let base = units.base_unit().base_unit();
        let accounts = crate::adapter::accounts::budget_accounts(&accounts.accounts().accounts());
        BudgetsAdapter::seed(
            &writer,
            vec![lib_budgets::personal_spending(base, &accounts)],
        )?;
        tracing::info!(path = %path.display(), "Gave a Ledger with no Budget a default one");
    }
    let budgets = BudgetsAdapter::load(pending.clone())?;
    let inventory = InventoryAdapter::load(pending.clone())?;
    if DocumentsAdapter::has_no_types(&writer)? {
        DocumentsAdapter::seed(&writer, lib_documents::types::default_types(), Vec::new())?;
        tracing::info!(path = %path.display(), "Gave a Ledger with no Document Type the defaults");
    }
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

    /// The stub Ledger created at `path` as of [`today`], opened.
    fn seeded(path: &Path) -> Storage {
        seed::stub_ledger(path, today()).expect("the stub Ledger is created");
        open_storage(path).expect("a new Ledger opens")
    }

    #[test]
    fn a_new_ledger_opens_seeded_and_a_write_survives_a_restart() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");

        let mut storage = seeded(&path);
        let seeded = storage.transactions.transactions();
        assert_eq!(seeded, seed::stub_transactions(today()));
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
    fn a_new_ledger_opens_with_the_stub_budgets_and_an_edit_survives_a_restart() {
        use lib_budgets::BudgetRepository as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.budgets.budgets(), seed::stub_budgets(today()));
        let mut joint = storage
            .budgets
            .budget(lib_core::BudgetId::seed(2))
            .expect("a stub Budget");
        joint.name = "Shared house".into();
        storage.budgets.upsert_budget(&joint);
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);

        let reopened = open_storage(&path).expect("the Ledger opens again");
        let mut expected = seed::stub_budgets(today());
        expected[1] = joint;
        assert_eq!(reopened.budgets.budgets(), expected);
    }

    /// A Ledger written before Budgets persisted has none, so it opens with an empty default
    /// Personal spending Budget, once.
    #[test]
    fn an_existing_ledger_with_no_budget_opens_with_an_empty_default_one() {
        use lib_budgets::BudgetRepository as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let storage = seeded(&path);
        storage
            .writer
            .run(|pool| async move {
                sqlx::query("DELETE FROM budgets").execute(&pool).await?;
                Ok(())
            })
            .expect("the Budgets are cleared");
        drop(storage);

        let reopened = open_storage(&path).expect("the Ledger opens again");
        let accounts = crate::adapter::accounts::budget_accounts(&seed::stub_accounts());
        let personal = lib_budgets::personal_spending(lib_core::UnitId::SEED_AUD, &accounts);
        assert!(personal.limits.is_empty() && personal.is_default);
        assert_eq!(reopened.budgets.budgets(), std::slice::from_ref(&personal));
        reopened.writer.flush().expect("the writer is running");
        drop(reopened);

        let again = open_storage(&path).expect("the Ledger opens again");
        assert_eq!(again.budgets.budgets(), [personal], "seeded once");
    }

    /// A Ledger written before Documents persisted has no Document Type, so it opens with the
    /// defaults, once.
    #[test]
    fn an_existing_ledger_with_no_document_type_opens_with_the_defaults() {
        use lib_documents::DocumentRepository as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let storage = seeded(&path);
        storage
            .writer
            .run(|pool| async move {
                sqlx::query("DELETE FROM document_links")
                    .execute(&pool)
                    .await?;
                sqlx::query("DELETE FROM documents").execute(&pool).await?;
                sqlx::query("DELETE FROM document_types")
                    .execute(&pool)
                    .await?;
                Ok(())
            })
            .expect("the Documents and their Types are cleared");
        drop(storage);

        let reopened = open_storage(&path).expect("the Ledger opens again");
        assert_eq!(
            reopened.documents.document_types(),
            lib_documents::types::default_types()
        );
        reopened.writer.flush().expect("the writer is running");
        drop(reopened);

        let again = open_storage(&path).expect("the Ledger opens again");
        assert_eq!(
            again.documents.document_types().len(),
            lib_documents::types::default_types().len(),
            "seeded once"
        );
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
    fn a_new_ledger_opens_with_the_stub_institutions_and_an_added_one_survives_a_restart() {
        use lib_institutions::{Institution, InstitutionRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.institutions.institutions(),
            seed::stub_institutions()
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

        let mut expected = seed::stub_institutions();
        expected.push(bank);
        assert_eq!(reopened.institutions.institutions(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_accounts_and_an_added_one_survives_a_restart() {
        use lib_accounts::Account;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.accounts.accounts().accounts(),
            seed::stub_accounts()
        );
        let savings = Account {
            id: lib_core::AccountId::new(),
            name: "Holiday Fund".into(),
            ..seed::stub_accounts()[1].clone()
        };

        storage
            .accounts
            .accounts_mut()
            .upsert_account(savings.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path).expect("the Ledger opens again");

        let mut expected = seed::stub_accounts();
        expected.push(savings);
        assert_eq!(reopened.accounts.accounts().accounts(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_categories_and_an_added_one_survives_a_restart() {
        use lib_categories::{Category, CategoryRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.categories.categories(), seed::stub_categories());
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
            seed::stub_categories().len() + 1
        );
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_tags_and_an_added_one_survives_a_restart() {
        use lib_tags::Tag;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.tags.tags().tags(), seed::stub_tags());
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
            seed::stub_tags().len() + 1
        );
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_inventory_and_an_added_location_survives_a_restart() {
        use lib_inventory::interactor::{AddArea, AddItem, AddLocation};
        use lib_inventory::{ItemDraft, LocationDraft};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(
            storage.inventory.inventory().inventory(),
            seed::stub_inventory(today())
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
    fn a_new_ledger_opens_with_the_stub_payees_and_an_added_one_survives_a_restart() {
        use lib_payees::Payee;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = seeded(&path);
        assert_eq!(storage.payees.payees().payees(), seed::stub_payees());
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
    fn a_new_ledger_opens_with_the_stub_bills_and_their_paid_entries() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let storage = seeded(&dir.path().join("ledger.pldb"));

        let expected = seed::stub_bills(today());
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
            "the stub's Paid entries keep their Transactions"
        );
    }
}
