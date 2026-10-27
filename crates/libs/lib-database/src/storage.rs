//! A Client's Ledger file, opened at start-up: the shared writer on the `[Personal-Ledger] file`,
//! the Client migrations, then the Units, Institutions, Accounts, Categories, Tags, Payees,
//! Transactions, Bills and Inventory adapters on the pending batch every adapter shares. The Desktop and the TUI both open it here,
//! so they read and write the same rows (ADR-0046), and both build their Shell on the [`Stores`]
//! made from it.

use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;

use chrono::{NaiveDate, Utc};
use lib_units::fake::InMemoryPorts;

use crate::{
    AccountsAdapter, BillsAdapter, CategoriesAdapter, Error, InstitutionsAdapter, InventoryAdapter,
    PayeesAdapter, PendingWrites, Result, TagsAdapter, TransactionsAdapter, UnitsAdapter,
    WriteFailure, Writer, adapter::store, seed,
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
    pub inventory: InventoryAdapter,
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
    pub inventory: Box<dyn store::InventoryStore>,
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
            inventory: Box::new(storage.inventory),
            commit: Rc::new(storage.pending),
            failures: storage.failures,
        }
    }
}

/// Opens the Ledger at `path`, creating and migrating it if need be, and loads every adapter,
/// blocking. A Ledger with no base Unit is new, so it is seeded first with the stub's Units, Price
/// Sources and base Unit, which the stub Accounts, Bill Plans and Budgets still name (#746), the
/// stub's Institutions, which its Accounts name (#849), the stub's Accounts, which its
/// Transactions and Bill Plans name (#853), the stub's Categories, which its Payees, Splits and
/// Bill Plans name (#858), the stub's Tags, which its Splits carry (#862), the stub's Payees, which its Splits and Bill Plans name, the stub Ledger's Transactions and Bills as of `today`, whose Paid entries and
/// Documents name those Transactions (#774), and last the stub's Inventory as of `today`, whose
/// Items the stub Documents link (#866). The first-launch seed is removed once every domain
/// persists.
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path, today: NaiveDate) -> Result<Storage> {
    let (writer, failures) = Writer::open_ledger(path)?;
    let pending = PendingWrites::new(writer.clone());
    let units = match UnitsAdapter::load(pending.clone()) {
        Err(Error::NotFound(_)) => {
            UnitsAdapter::seed_from(&writer, InMemoryPorts::stub(Utc::now()))?;
            InstitutionsAdapter::seed(&writer, seed::stub_institutions())?;
            AccountsAdapter::seed(&writer, seed::stub_accounts())?;
            CategoriesAdapter::seed(&writer, seed::stub_categories())?;
            TagsAdapter::seed(&writer, seed::stub_tags())?;
            PayeesAdapter::seed(&writer, seed::stub_payees())?;
            TransactionsAdapter::seed(&writer, seed::stub_ledger_transactions(today))?;
            let bills = seed::stub_ledger_bills(today);
            BillsAdapter::seed(&writer, bills.plans, bills.entries)?;
            InventoryAdapter::seed(&writer, seed::stub_inventory(today))?;
            tracing::info!(path = %path.display(), "Seeded a new Ledger with the stub");
            UnitsAdapter::load(pending.clone())?
        }
        loaded => loaded?,
    };
    let institutions = InstitutionsAdapter::load(pending.clone())?;
    let accounts = AccountsAdapter::load(pending.clone())?;
    let categories = CategoriesAdapter::load(pending.clone())?;
    let tags = TagsAdapter::load(pending.clone())?;
    let payees = PayeesAdapter::load(pending.clone())?;
    let transactions = TransactionsAdapter::load(pending.clone())?;
    let bills = BillsAdapter::load(pending.clone())?;
    let inventory = InventoryAdapter::load(pending.clone())?;
    Ok(Storage {
        units,
        institutions,
        accounts,
        categories,
        tags,
        payees,
        transactions,
        bills,
        inventory,
        pending,
        writer,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use lib_bills::{BillScheduleRepository, Resolution};
    use lib_transactions::TransactionRepository;

    use super::*;
    use crate::Commit;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    #[test]
    fn a_new_ledger_opens_seeded_and_a_write_survives_a_restart() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");

        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
        let seeded = storage.transactions.transactions();
        assert_eq!(seeded, seed::stub_ledger_transactions(today()));
        let gone = seeded[0].id;
        storage.transactions.delete_transaction(gone);
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);

        // A day later, so a re-seed would show as shifted dates.
        let reopened = open_storage(&path, today().succ_opt().expect("a real date"))
            .expect("the Ledger opens again");

        let mut expected = seeded;
        expected.remove(0);
        assert_eq!(reopened.transactions.transactions(), expected);
        assert!(reopened.transactions.transaction(gone).is_none());
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_institutions_and_an_added_one_survives_a_restart() {
        use lib_institutions::{Institution, InstitutionRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
        assert_eq!(
            storage.institutions.institutions(),
            seed::stub_institutions()
        );
        let bank = Institution {
            id: lib_core::InstitutionId::new(),
            name: "Bank X".into(),
            products: "savings".into(),
        };

        storage.institutions.add_institution(bank.clone());
        storage.pending.commit();
        storage.writer.flush().expect("the writer is running");
        drop(storage);
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

        let mut expected = seed::stub_institutions();
        expected.push(bank);
        assert_eq!(reopened.institutions.institutions(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_accounts_and_an_added_one_survives_a_restart() {
        use lib_accounts::Account;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
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
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

        let mut expected = seed::stub_accounts();
        expected.push(savings);
        assert_eq!(reopened.accounts.accounts().accounts(), expected);
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_categories_and_an_added_one_survives_a_restart() {
        use lib_categories::{Category, CategoryRepository};

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
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
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

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
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
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
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

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
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
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
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

        let reread = reopened.inventory.inventory().inventory();
        assert_eq!(reread, written);
        assert_eq!(reread.item(kayak).map(|item| item.area), Some(deck));
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_payees_and_an_added_one_survives_a_restart() {
        use lib_payees::Payee;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("ledger.pldb");
        let mut storage = open_storage(&path, today()).expect("a new Ledger opens");
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
        let reopened = open_storage(&path, today()).expect("the Ledger opens again");

        assert_eq!(reopened.payees.payees().payee(corner.id), Some(corner));
    }

    #[test]
    fn a_new_ledger_opens_with_the_stub_bills_and_their_paid_entries() {
        let dir = tempfile::tempdir().expect("a temporary directory");

        let storage =
            open_storage(&dir.path().join("ledger.pldb"), today()).expect("a new Ledger opens");

        let expected = seed::stub_ledger_bills(today());
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
