//! The dev seed: each persisted domain's stub seed, written through its adapter. A module's stub
//! seed is the one seed for its domain, so a change to a persisted entity updates that seed and
//! this follows. Ids match `lib-ledger`'s stub, which the domains not yet persisted still read.
//!
//! The Budgets are `lib-budgets`' stub, fed the stub Accounts and Categories as Budgets reads them.
//!
//! The Transactions are `lib_transactions::fake`'s booked stub, the generated history then the
//! rows the Bills' Paid entries and the Documents' Links name (#961), and the Bills and Documents
//! are their own crates' stubs, each fed the rows it names as it reads them. No seed comes from
//! `lib-ledger`'s joined stub (#963), and no seed writes another domain.
//!
//! Every write is an upsert, so seeding twice leaves the same rows. The base Unit is the exception:
//! it is written only when the Ledger has none, so a base the person changed survives a re-seed.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use fake::faker::company::en::CompanyName;
use fake::faker::currency::en::CurrencyCode;
use fake::rand::rngs::StdRng;
use fake::rand::{Rng, SeedableRng};
use fake::{Fake, Faker};
use lib_bills::{AmountKind, BillPlan, BillsSeed, Recurrence};
use lib_core::{
    BillPlanId, CategoryId, Money, PayeeId, TagId, TransactionId, TransactionStatus, UnitId,
    UnitKind,
};
use lib_transactions::{Split, Transaction};
use lib_units::fake::InMemoryPorts;
use lib_units::{Precision, PriceLink, UnitRow, seed_weekly_prices};

use crate::Result;
use crate::adapter::accounts::AccountsAdapter;
use crate::adapter::bills::BillsAdapter;
use crate::adapter::budgets::BudgetsAdapter;
use crate::adapter::categories::CategoriesAdapter;
use crate::adapter::documents::DocumentsAdapter;
use crate::adapter::institutions::InstitutionsAdapter;
use crate::adapter::inventory::InventoryAdapter;
use crate::adapter::payees::PayeesAdapter;
use crate::adapter::tags::TagsAdapter;
use crate::adapter::transactions::TransactionsAdapter;
use crate::adapter::units::UnitsAdapter;
use crate::writer::Writer;

/// Writes every persisted domain's stub seed in one transaction, blocking until it lands: Units,
/// Price Sources and the base Unit, the Institutions, which the Accounts name, the Accounts, which
/// the Transactions and Bill Plans name, the Categories, which the Payees, Splits and Bill Plans
/// name, the Tags, which the Splits name (#862), the Payees, which the Splits and Bill Plans name,
/// then the Transactions and the Bills dated from `now`'s local day, the Budgets as of that day,
/// which name the Units, Accounts and Categories (#952), the Inventory as of that day (#866), and
/// last the Document Types and Documents as of that day, whose Links name the rows before them
/// (#963), in the order Units, Institutions, Accounts, Categories, Tags, Payees, Transactions,
/// Bills, Budgets, Inventory, Documents (Q17).
///
/// # Errors
/// Returns the error of a write; nothing is written then.
#[tracing::instrument(name = "Seed the stub", level = "debug", skip(writer), err)]
pub fn stub(writer: &Writer, now: DateTime<Utc>) -> Result<()> {
    write_stub(writer, now, now.with_timezone(&Local).date_naive(), None)
}

/// Writes [`stub`]'s seed in one transaction, its Units' prices as of `now` and its Transactions,
/// Bills, Inventory and Documents as of `today`, with `personal` in place of the stub's Personal Institution
/// when given. All or nothing, so a seed that fails partway, or a process that dies in it, never
/// leaves a Ledger with a base Unit and only some of the rest (#886). The one place the domains'
/// order lives: each names rows of those before it.
///
/// # Errors
/// Returns the error of a write; nothing is written then.
pub(crate) fn write_stub(
    writer: &Writer,
    now: DateTime<Utc>,
    today: NaiveDate,
    personal: Option<lib_institutions::Institution>,
) -> Result<()> {
    write_stub_rows(writer, now, today, personal, true)
}

/// [`write_stub`], with the Documents left out unless `documents`, for an adapter test that writes
/// its own over the rest of the stub.
pub(crate) fn write_stub_rows(
    writer: &Writer,
    now: DateTime<Utc>,
    today: NaiveDate,
    personal: Option<lib_institutions::Institution>,
    documents: bool,
) -> Result<()> {
    let units = InMemoryPorts::stub(now);
    let institutions: Vec<_> = stub_institutions()
        .into_iter()
        .map(|institution| match &personal {
            Some(personal) if personal.id == institution.id => personal.clone(),
            _ => institution,
        })
        .collect();
    let accounts = stub_accounts();
    let categories = stub_categories();
    let tags = stub_tags();
    let payees = stub_payees();
    let transactions = stub_transactions(today);
    let bills = stub_bills(today);
    let budgets = stub_budgets(today);
    let inventory = stub_inventory(today);
    let (types, documents) = if documents {
        (
            lib_documents::types::default_types(),
            stub_documents(today, &institutions),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    writer.run(move |pool| async move {
        let mut conn = pool.begin().await?;
        // A seed's rows may name one written after them.
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *conn)
            .await?;
        UnitsAdapter::seed_in(
            &mut conn,
            &units.units.units,
            &units.price_sources.records,
            units.base_unit.0,
        )
        .await?;
        InstitutionsAdapter::seed_in(&mut conn, &institutions).await?;
        AccountsAdapter::seed_in(&mut conn, &accounts).await?;
        CategoriesAdapter::seed_in(&mut conn, &categories).await?;
        TagsAdapter::seed_in(&mut conn, &tags).await?;
        PayeesAdapter::seed_in(&mut conn, &payees).await?;
        TransactionsAdapter::seed_in(&mut conn, &transactions).await?;
        BillsAdapter::seed_in(&mut conn, &bills.plans, &bills.entries).await?;
        BudgetsAdapter::seed_in(&mut conn, &budgets).await?;
        InventoryAdapter::seed_in(&mut conn, &inventory).await?;
        DocumentsAdapter::seed_in(&mut conn, &types, &documents).await?;
        conn.commit().await?;
        Ok(())
    })
}

/// `lib-institutions`' stub seed, the stub Ledger's Institutions, with the ids its Accounts name.
pub fn stub_institutions() -> Vec<lib_institutions::Institution> {
    lib_institutions::fake::InMemoryInstitutions::stub().institutions
}

/// `lib-accounts`' stub seed, the stub Ledger's Accounts, with the ids its Transactions, Bill Plans
/// and Budgets name.
pub fn stub_accounts() -> Vec<lib_accounts::Account> {
    lib_accounts::fake::InMemoryAccounts::stub().accounts
}

/// `lib-categories`' stub seed, the stub Ledger's Categories, with the ids its Splits, Bill Plans,
/// Payees and Budgets name.
pub fn stub_categories() -> Vec<lib_categories::Category> {
    lib_categories::fake::InMemoryCategories::stub().categories
}

/// `lib-budgets`' stub seed as of `today`, the stub Ledger's Budgets, fed the stub Accounts and
/// Categories as Budgets reads them.
pub fn stub_budgets(today: NaiveDate) -> Vec<lib_budgets::Budget> {
    let accounts = crate::adapter::accounts::budget_accounts(&stub_accounts());
    let categories = crate::adapter::categories::budget_categories(&stub_categories());
    lib_budgets::default_budgets(&accounts, &categories, today)
        .all()
        .to_vec()
}

/// `lib-tags`' stub seed, the stub Ledger's Tags, with the ids its Splits carry.
pub fn stub_tags() -> Vec<lib_tags::Tag> {
    lib_tags::fake::InMemoryTags::stub().tags
}

/// `lib-inventory`'s stub seed as of `today`, the stub Ledger's Inventory, with the Item ids its
/// Documents link.
pub fn stub_inventory(today: NaiveDate) -> lib_inventory::Inventory {
    let lib_inventory::fake::InMemoryInventory { locations, items } =
        lib_inventory::fake::InMemoryInventory::stub(today);
    lib_inventory::Inventory { locations, items }
}

/// `lib-payees`' stub seed, the stub Ledger's Payees, with the ids its Splits and Bill Plans name.
pub fn stub_payees() -> Vec<lib_payees::Payee> {
    lib_payees::fake::InMemoryPorts::stub().payees.payees
}

/// The names `lib_transactions::fake`'s stub books by, looked up in the per-domain stubs.
struct StubNames {
    categories: Vec<lib_categories::Category>,
    payees: Vec<lib_payees::Payee>,
    tags: Vec<lib_tags::Tag>,
}

impl lib_transactions::fake::SeedNames for StubNames {
    fn category(&self, name: &str) -> Option<CategoryId> {
        lib_categories::find_by_name(&self.categories, name)
    }

    fn payee(&self, name: &str) -> Option<PayeeId> {
        lib_payees::find_by_name(&self.payees, name)
    }

    fn tag(&self, name: &str) -> Option<TagId> {
        lib_tags::find_by_name(&self.tags, name)
    }
}

/// `lib_transactions::fake`'s booked stub as of `today` over the per-domain stubs: the generated
/// history, then the rows the Bills and Documents stubs name, numbered from
/// [`lib_transactions::fake::FIRST_BOOKED`].
pub fn stub_transactions(today: NaiveDate) -> Vec<Transaction> {
    let accounts: Vec<_> = stub_accounts()
        .into_iter()
        .map(|account| lib_transactions::fake::SeedAccount {
            transaction_count: lib_transactions::fake::stub_transaction_count(account.id),
            id: account.id,
            name: account.name,
            account_type: account.account_type,
        })
        .collect();
    let names = StubNames {
        categories: stub_categories(),
        payees: stub_payees(),
        tags: stub_tags(),
    };
    lib_transactions::fake::booked_stub_transactions(&accounts, &names, today)
}

/// `lib-bills`' stub Bill Plans and Bill Schedule as of `today`, fed the stub Accounts, Categories
/// and Payees as Bills reads them, whose Paid entries name [`stub_transactions`]' payments.
pub fn stub_bills(today: NaiveDate) -> BillsSeed {
    lib_bills::default_bills(
        &crate::adapter::accounts::bill_accounts(&stub_accounts()),
        &crate::adapter::categories::bill_categories(&stub_categories()),
        &crate::adapter::payees::bill_payees(&stub_payees()),
        lib_transactions::fake::FIRST_BOOKED,
        today,
    )
    .seed
}

/// `lib-documents`' stub Documents as of `today`, fed the stub rows their Links name as the stores
/// answer Documents' read ports, the Accounts' Institutions from `institutions`.
pub fn stub_documents(
    today: NaiveDate,
    institutions: &[lib_institutions::Institution],
) -> Vec<lib_documents::Document> {
    let records = lib_documents::LinkRecords {
        transactions: crate::adapter::transactions::document_transactions(&stub_transactions(
            today,
        )),
        payees: crate::adapter::payees::document_payees(&stub_payees()),
        accounts: crate::adapter::accounts::document_accounts(&stub_accounts(), institutions),
        plans: crate::adapter::bills::document_bill_plans(&stub_bills(today).plans),
        items: crate::adapter::inventory::document_items(&stub_inventory(today).items),
    };
    lib_documents::default_documents(&records, today).documents
}

/// Fixed so the generated rows are the same on every run, which keeps `fake` seeding idempotent.
const FAKE_RNG_SEED: u64 = 516;

/// Where the generated Units' seed numbers start. The stub's own Units use 1 to 3, so the two never
/// share an id.
const FAKE_FIRST_NUMBER: u32 = 100;

/// `count` generated Units, quoted in the stub's base Unit. The same `count` always gives the same
/// ids, codes and names, so a second seed upserts over the first rather than adding rows. Only the
/// Weekly Prices' weeks and newest write follow `now`.
pub fn fake_units(count: u32, now: DateTime<Utc>) -> Vec<UnitRow> {
    let mut rng = StdRng::seed_from_u64(FAKE_RNG_SEED);
    let kinds = [
        UnitKind::Currency,
        UnitKind::Crypto,
        UnitKind::Equity,
        UnitKind::FixedIncome,
        UnitKind::Commodity,
        UnitKind::Custom,
    ];
    (0..count)
        .map(|index| {
            let number = FAKE_FIRST_NUMBER + index;
            let kind = kinds[rng.random_range(0..kinds.len())];
            // The number keeps the code unique, which the faker's short list can't.
            let code = format!(
                "{}{number}",
                CurrencyCode()
                    .fake_with_rng::<String, _>(&mut rng)
                    .to_lowercase()
            );
            // A priced Unit's last price, then a year of Weekly Prices leading up to it.
            let last_price = (kind != UnitKind::Currency).then(|| {
                (
                    rng.random_range(1..=2_000_000),
                    lib_units::seed_written_at(now, Duration::minutes(rng.random_range(1..=1_440))),
                )
            });
            UnitRow {
                id: UnitId::seed(number),
                code,
                name: CompanyName().fake_with_rng(&mut rng),
                kind,
                precision: Precision::of_kind(kind),
                link: PriceLink::manual(UnitId::SEED_AUD),
                prices: last_price.map_or_else(Vec::new, |(cents, written_at)| {
                    seed_weekly_prices(cents, written_at, number)
                }),
                is_active: Faker.fake_with_rng(&mut rng),
            }
        })
        .collect()
}

/// Where the generated Transactions' seed numbers start, well clear of the stub's, which number
/// up from 1 to a little over its 700.
const FAKE_FIRST_TRANSACTION: u32 = 100_000;

/// `count` generated Transactions dated within the stub's history before `today`. Each copies the
/// Account and one Split's Category, Payee and Tags from a stub Transaction, so it names rows the
/// stub Ledger has and keeps the sign its Category takes. The same `count` and `today` always give
/// the same rows, so a second seed upserts over the first rather than adding rows.
pub fn fake_transactions(count: u32, today: NaiveDate) -> Vec<Transaction> {
    let stub = stub_transactions(today);
    let mut rng = StdRng::seed_from_u64(FAKE_RNG_SEED);
    (0..count)
        .filter_map(|index| {
            let model = stub.get(rng.random_range(0..stub.len()))?;
            let line = model.splits.get(rng.random_range(0..model.splits.len()))?;
            let cents: i64 = rng.random_range(100..=50_000);
            let sign = if line.amount.0 < 0 { -1 } else { 1 };
            let statuses = [
                TransactionStatus::Open,
                TransactionStatus::Cleared,
                TransactionStatus::Reconciled,
            ];
            Some(Transaction {
                id: TransactionId::seed(FAKE_FIRST_TRANSACTION + index),
                date: today - Duration::days(rng.random_range(0..lib_ledger::HISTORY_DAYS)),
                account_id: model.account_id,
                status: statuses[rng.random_range(0..statuses.len())].clone(),
                is_flagged: rng.random_ratio(1, 20),
                description: Some(CompanyName().fake_with_rng(&mut rng)),
                splits: vec![Split {
                    amount: Money(BigDecimal::new((sign * cents).into(), 2)),
                    ..line.clone()
                }],
            })
        })
        .collect()
}

/// Where the generated Bill Plans' seed numbers start, clear of the stub's, which number up from 1.
/// Below 4096, so their entries' seed ids stay unique ([`lib_bills::seed_entry_id`]).
const FAKE_FIRST_BILL_PLAN: u32 = 1_000;

/// `count` generated Bill Plans and their generated entries as of `today`, all open. Each copies
/// the Account, Unit, Category and Payee of a stub Bill Plan, so it names rows the stub Ledger has.
/// The same `count` and `today` always give the same rows, so a second seed upserts over the
/// first rather than adding rows.
pub fn fake_bills(count: u32, today: NaiveDate) -> BillsSeed {
    let stub = stub_bills(today).plans;
    let mut rng = StdRng::seed_from_u64(FAKE_RNG_SEED);
    let plans: Vec<BillPlan> = (0..count)
        .filter_map(|index| {
            let model = stub.get(rng.random_range(0..stub.len()))?;
            let recurrence = Recurrence::ALL[rng.random_range(0..Recurrence::ALL.len())];
            let first_due = today - Duration::days(rng.random_range(0..60));
            Some(BillPlan {
                id: BillPlanId::seed(FAKE_FIRST_BILL_PLAN + index),
                name: CompanyName().fake_with_rng(&mut rng),
                planned_amount: Money(BigDecimal::new(rng.random_range(500..=50_000).into(), 2)),
                amount_kind: if rng.random_ratio(1, 2) {
                    AmountKind::Fixed
                } else {
                    AmountKind::Estimated
                },
                recurrence,
                first_due,
                ends_on: None,
                attention_lead: Some(rng.random_range(0..=7)),
                is_active: true,
                generate_from: first_due,
                ..model.clone()
            })
        })
        .collect();
    let mut entries = Vec::new();
    lib_bills::populate(&plans, &mut entries, today);
    for entry in &mut entries {
        entry.row_id = lib_bills::seed_entry_id(entry.plan_id, entry.due);
    }
    BillsSeed { plans, entries }
}

/// How many generated Bill Plans go with `count` generated Units and Transactions: a Ledger holds
/// far fewer Bill Plans than Transactions.
fn fake_bill_count(count: u32) -> u32 {
    count.div_ceil(5)
}

/// Writes `count` generated Units, `count` generated Transactions and a fifth as many generated
/// Bill Plans on top of the stub seed, blocking until they land. Run after [`stub`]: the Units are
/// quoted in its base Unit, and the Transactions and Bill Plans name its Accounts, Categories,
/// Payees and Tags.
///
/// # Errors
/// Returns the error of a write; nothing of that domain's is written then.
#[tracing::instrument(name = "Seed the generated rows", level = "debug", skip(writer), err)]
pub fn fake(writer: &Writer, now: DateTime<Utc>, count: u32) -> Result<()> {
    let today = now.with_timezone(&Local).date_naive();
    UnitsAdapter::seed(writer, fake_units(count, now), Vec::new(), UnitId::SEED_AUD)?;
    TransactionsAdapter::seed(writer, fake_transactions(count, today))?;
    let bills = fake_bills(fake_bill_count(count), today);
    BillsAdapter::seed(writer, bills.plans, bills.entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::{Commit, PendingWrites};
    use chrono::TimeZone;

    fn temp_writer() -> (tempfile::TempDir, Writer) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (writer, _failures) =
            Writer::open_ledger(&dir.path().join("ledger.db")).expect("a new Ledger opens");
        (dir, writer)
    }

    fn stored_transactions(writer: &Writer) -> Vec<Transaction> {
        let adapter = TransactionsAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Transactions load");
        lib_transactions::contract::snapshot(&adapter)
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    fn stored_bills(writer: &Writer) -> (Vec<BillPlan>, Vec<lib_bills::BillScheduleEntry>) {
        let adapter =
            BillsAdapter::load(PendingWrites::new(writer.clone())).expect("the Bills load");
        lib_bills::contract::snapshot(&adapter)
    }

    fn stored_payees(writer: &Writer) -> Vec<lib_payees::Payee> {
        let adapter =
            PayeesAdapter::load(PendingWrites::new(writer.clone())).expect("the Payees load");
        let mut payees = lib_payees::PayeeRepository::payees(adapter.payees());
        payees.sort_by_key(|payee| payee.id);
        for payee in &mut payees {
            payee.aliases.sort();
        }
        payees
    }

    /// The Institutions are the stub Ledger's, by id and in order, so the Accounts naming them
    /// name stored rows.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_institutions_twice_over_the_same_rows() {
        use lib_institutions::InstitutionRepository;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored = InstitutionsAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Institutions load");
        assert_eq!(
            stored.institutions(),
            lib_institutions::default_institutions()
        );
    }

    /// The Personal Institution is the person's once the Ledger exists, so `db-seed` leaves a name
    /// they gave it, while the stub's other Institutions are still put back.
    #[test]
    fn seeding_again_keeps_a_renamed_personal_institution() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the first seed lands");
        writer
            .run(|pool| async move {
                sqlx::query("UPDATE institutions SET name = 'Persönlich' WHERE id = ?")
                    .bind(lib_core::InstitutionId::SEED_PERSONAL)
                    .execute(&pool)
                    .await?;
                sqlx::query("UPDATE institutions SET name = 'Renamed' WHERE id = ?")
                    .bind(lib_core::InstitutionId::SEED_ANZ)
                    .execute(&pool)
                    .await?;
                Ok(())
            })
            .expect("the rows are renamed");

        stub(&writer, now).expect("the second seed lands");

        let stored = InstitutionsAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Institutions load");
        let name = |id| {
            use lib_institutions::InstitutionRepository;
            stored.institution(id).expect("a stub row").name
        };
        assert_eq!(name(lib_core::InstitutionId::SEED_PERSONAL), "Persönlich");
        assert_eq!(name(lib_core::InstitutionId::SEED_ANZ), "ANZ Banking Group");
    }

    /// The Accounts are the stub Ledger's, by id and in order, so the Transactions and Bill Plans
    /// naming them name stored rows.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_accounts_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored =
            AccountsAdapter::load(PendingWrites::new(writer.clone())).expect("the Accounts load");
        assert_eq!(
            stored.accounts().accounts(),
            lib_ledger::Ledger::stub().accounts()
        );
    }

    /// The Tags are the stub Ledger's, by id, so the Splits written after them carry stored rows.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_tags_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored = TagsAdapter::load(PendingWrites::new(writer.clone())).expect("the Tags load");
        assert_eq!(stored.tags().tags(), lib_tags::default_tags());
    }

    /// The Inventory is the stub Ledger's as of the seed's day, by id, so the stub Documents' Links
    /// name stored Items, and it is written last, after the Bills.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_inventory_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored = InventoryAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Inventory loads");
        let today = now.with_timezone(&Local).date_naive();
        assert_eq!(
            stored.inventory().inventory(),
            lib_inventory::default_inventory(today)
        );
    }

    /// The Document Types and Documents are the stub Ledger's as of the seed's day, written last,
    /// over the rows their Links name, so every Link is kept (#963).
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_documents_twice_over_the_same_rows() {
        use lib_documents::DocumentRepository as _;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored =
            DocumentsAdapter::load(PendingWrites::new(writer.clone())).expect("the Documents load");
        let today = now.with_timezone(&Local).date_naive();
        let ledger = lib_ledger::Ledger::stub_on(today);
        assert_eq!(
            stored.document_types(),
            lib_documents::types::default_types()
        );
        assert_eq!(stored.documents(), ledger.stub_documents(today));
    }

    /// The seed builds the stub Transactions and Bills from the per-domain stubs itself, not from
    /// `lib-ledger`'s joined stub, so the two are held to the same rows on several days (#963).
    #[test]
    fn the_seeds_own_stubs_are_the_stub_ledgers() {
        for today in [
            today(),
            NaiveDate::from_ymd_opt(2026, 1, 31).expect("a real date"),
            NaiveDate::from_ymd_opt(2028, 2, 29).expect("a real date"),
        ] {
            let ledger = lib_ledger::Ledger::stub_on(today);
            assert_eq!(
                stub_transactions(today),
                ledger.transactions(),
                "on {today}"
            );
            let bills = stub_bills(today);
            assert_eq!(bills.plans, ledger.bill_plans(), "on {today}");
            assert_eq!(bills.entries, ledger.bill_entries(), "on {today}");
            assert_eq!(
                stub_documents(today, &stub_institutions()),
                ledger.stub_documents(today),
                "on {today}"
            );
        }
    }

    /// The Budgets are the stub Ledger's as of the seed's day, by id, so Categories 5c finds
    /// Personal spending by `PERSONAL_SPENDING_ID`.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_budgets_twice_over_the_same_rows() {
        use lib_budgets::BudgetRepository as _;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        let stored =
            BudgetsAdapter::load(PendingWrites::new(writer.clone())).expect("the Budgets load");
        let today = now.with_timezone(&Local).date_naive();
        assert_eq!(
            stored.budgets(),
            lib_ledger::Ledger::stub_on(today).budgets().all()
        );
    }

    /// The Payees are the stub Ledger's, by id, so the Splits and Bill Plans written after them
    /// name stored rows.
    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_payees_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let mut expected = lib_payees::default_payees();
        expected.sort_by_key(|payee| payee.id);
        for payee in &mut expected {
            payee.aliases.sort();
        }

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        assert_eq!(stored_payees(&writer), expected);
    }

    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_bills_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let expected = stub_bills(now.with_timezone(&Local).date_naive());

        stub(&writer, now).expect("the first seed lands");
        let first = stored_bills(&writer);
        stub(&writer, now).expect("the second seed lands");

        assert_eq!(first.0.len(), expected.plans.len());
        assert_eq!(first.1.len(), expected.entries.len());
        assert_eq!(stored_bills(&writer), first);
    }

    #[test]
    fn seeding_generated_bills_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the stub lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = stored_bills(&writer);

        fake(&writer, now, 20).expect("the second generated seed lands");

        let stub = stub_bills(now.with_timezone(&Local).date_naive());
        assert_eq!(first.0.len(), stub.plans.len() + 4);
        assert!(first.1.len() > stub.entries.len());
        assert_eq!(stored_bills(&writer), first);
    }

    #[test]
    fn generated_bills_are_the_same_every_time_and_clear_of_the_stub() {
        let first = fake_bills(4, today());
        let stub_ids: Vec<_> = stub_bills(today())
            .plans
            .iter()
            .map(|plan| plan.id)
            .collect();

        assert_eq!(first.plans.len(), 4);
        assert_eq!(first, fake_bills(4, today()));
        assert!(first.plans.iter().all(|plan| !stub_ids.contains(&plan.id)));
        assert!(!first.entries.is_empty());
    }

    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_transactions_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let mut expected = stub_transactions(now.with_timezone(&Local).date_naive());
        expected.sort_by_key(|transaction| transaction.id);

        stub(&writer, now).expect("the first seed lands");
        stub(&writer, now).expect("the second seed lands");

        assert!(
            expected.len() > 700,
            "the Bills and Documents stubs add theirs"
        );
        assert_eq!(stored_transactions(&writer), expected);
    }

    #[test]
    fn generated_transactions_are_the_same_every_time_and_clear_of_the_stub() {
        let first = fake_transactions(20, today());
        let stub_ids: Vec<_> = stub_transactions(today())
            .iter()
            .map(|transaction| transaction.id)
            .collect();

        assert_eq!(first.len(), 20);
        assert_eq!(first, fake_transactions(20, today()));
        assert!(
            first
                .iter()
                .all(|transaction| !stub_ids.contains(&transaction.id))
        );
    }

    #[test]
    fn seeding_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the first seed lands");
        let first = UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads");
        let units = first.units().units();
        let base = first.base_unit().base_unit();

        stub(&writer, now).expect("the second seed lands");
        let second =
            UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads again");

        assert!(!units.is_empty());
        assert_eq!(second.units().units(), units);
        assert_eq!(second.base_unit().base_unit(), base);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn seeding_again_keeps_a_changed_base_unit(
        _: sqlx::sqlite::SqlitePoolOptions,
        options: sqlx::sqlite::SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let now = Utc::now();
        stub(&writer, now).expect("the first seed lands");
        let pending = PendingWrites::new(writer.clone());
        let mut adapter = UnitsAdapter::load(pending.clone()).expect("the seed loads");
        assert_eq!(adapter.base_unit().base_unit(), UnitId::SEED_AUD);
        let changed = adapter
            .units()
            .units()
            .into_iter()
            .map(|unit| unit.id)
            .find(|id| *id != UnitId::SEED_AUD)
            .expect("the stub has a second Unit");
        let usage = lib_units::fake::InMemoryUnitUsage::default();
        adapter.split_mut(&usage).base_unit.set_base_unit(changed);
        pending.commit();

        stub(&writer, now).expect("the second seed lands");
        let reseeded =
            UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads again");

        assert_eq!(reseeded.base_unit().base_unit(), changed);
    }

    #[test]
    fn generated_units_are_the_same_every_time_and_clear_of_the_stub() {
        let now = Utc::now();
        let first = fake_units(20, now);
        let second = fake_units(20, now);
        let stub_ids: Vec<_> = InMemoryPorts::stub(now)
            .units
            .units
            .iter()
            .map(|unit| unit.id)
            .collect();

        assert_eq!(first, second);
        assert!(first.iter().all(|unit| !stub_ids.contains(&unit.id)));
        let mut codes: Vec<_> = first.iter().map(|unit| unit.code.clone()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), first.len());
    }

    #[test]
    fn seeding_generated_units_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the stub lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads");
        let units = first.units().units();

        fake(&writer, now, 20).expect("the second generated seed lands");
        let second =
            UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads again");

        assert_eq!(units.len(), 3 + 20);
        assert_eq!(second.units().units(), units);
    }

    /// Mondays are the week boundary: a seed written a few minutes into one must still price that
    /// week, not the Sunday's.
    #[test]
    fn every_seeded_priced_unit_loads_52_weeks_ending_on_the_current_week() {
        // 2026-11-02 is a Monday, so the week runs to the Sunday, the 8th.
        for (day, hour, minute) in [
            (2, 0, 3),
            (2, 12, 0),
            (3, 9, 30),
            (4, 23, 59),
            (5, 0, 0),
            (6, 18, 45),
            (7, 6, 15),
            (8, 23, 59),
            (9, 0, 1),
        ] {
            let now = Utc
                .with_ymd_and_hms(2026, 11, day, hour, minute, 0)
                .unwrap();
            let (_dir, writer) = temp_writer();
            stub(&writer, now).expect("the stub lands");
            fake(&writer, now, 20).expect("the generated seed lands");
            let units = UnitsAdapter::load(PendingWrites::new(writer.clone()))
                .expect("the seed loads")
                .units()
                .units();
            let current = lib_units::WeeklyPrice::week_of(now.date_naive());

            let priced: Vec<_> = units
                .iter()
                .filter(|unit| !unit.prices.is_empty())
                .collect();

            assert!(
                priced.iter().any(|unit| unit.id == UnitId::SEED_BTC),
                "the stub's btc on {now}"
            );
            assert!(priced.len() > 1, "and some generated Units on {now}");
            for unit in priced {
                assert_eq!(unit.prices.len(), 52, "{} on {now}", unit.code);
                assert_eq!(
                    unit.last_price().map(|price| price.week),
                    Some(current),
                    "{} on {now}",
                    unit.code
                );
            }
        }
    }

    #[test]
    fn seeding_generated_transactions_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the stub lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = stored_transactions(&writer);

        fake(&writer, now, 20).expect("the second generated seed lands");

        let stub = stub_transactions(now.with_timezone(&Local).date_naive());
        assert_eq!(first.len(), stub.len() + 20);
        assert_eq!(stored_transactions(&writer), first);
    }
}
