//! The Sample Ledger (`GLOSSARY.md`): one fixed, known Ledger for developing and testing Personal
//! Ledger, built from one definition, [`SampleLedger`]. It is `NewLedger`'s rows (the Personal
//! Institution, the base Unit, the Personal spending Budget and the default Document Types) with
//! each persisted domain's sample rows on top, every row dated from `today`. [`sample`] (`db-seed`)
//! writes it to the dev DB, [`sample_ledger`] to a new Ledger file and `FakeStores::sample` holds
//! it in memory, so the three hold the same rows with the same ids and a test fails if they drift.
//! A module's sample rows are the one sample for its domain, so a change to a persisted entity
//! updates them and this follows. The cross-domain seed tests sit beside it.
//!
//! The Budgets are `lib-budgets`' sample, fed the sample Accounts and Categories as Budgets reads
//! them, with Personal spending's Accounts and Limits on `NewLedger`'s.
//!
//! The Transactions are `lib_transactions::fake`'s booked sample, the generated history then the
//! rows the Bills' Paid entries and the Documents' Links name (#961), and the Bills and Documents
//! are their own crates' samples, each fed the rows it names as it reads them. No seed writes
//! another domain.
//!
//! Every write is an upsert, so seeding twice leaves the same rows. The base Unit is the exception:
//! it is written only when the Ledger has none, so a base the person changed survives a re-seed.

#[cfg(test)]
mod bills_seed_tests;
#[cfg(test)]
mod budgets_seed_tests;
#[cfg(test)]
mod documents_seed_tests;
#[cfg(test)]
mod transactions_seed_tests;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use fake::faker::company::en::CompanyName;
use fake::faker::currency::en::CurrencyCode;
use fake::rand::rngs::StdRng;
use fake::rand::{Rng, SeedableRng};
use fake::{Fake, Faker};
use lib_bills::{AmountKind, BillPlan, BillsSeed, Recurrence};
use lib_core::{
    AccountId, BillPlanId, CategoryId, Money, PayeeId, TagId, TransactionId, TransactionStatus,
    UnitId, UnitKind,
};
use lib_ledger_creation::NewLedger;
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

/// The name the Sample Ledger's Personal Institution is made with, as `:new` makes it in en-US.
const SAMPLE_PERSONAL: &str = "Personal";

/// Writes the Sample Ledger as of `now`'s local day in one transaction, blocking until it lands:
/// Units, Price Sources and the base Unit, the Institutions, which the Accounts name, the
/// Accounts, which the Transactions and Bill Plans name, the Categories, which the Payees, Splits
/// and Bill Plans name, the Tags, which the Splits name (#862), the Payees, which the Splits and
/// Bill Plans name, then the Transactions and the Bills, the Budgets, which name the Units,
/// Accounts and Categories (#952), the Inventory (#866), and last the Document Types and
/// Documents, whose Links name the rows before them (#963), in the order Units, Institutions,
/// Accounts, Categories, Tags, Payees, Transactions, Bills, Budgets, Inventory, Documents (Q17).
/// `NewLedger`'s rows are upserted with the rest, so a dev DB never made with `:new` still opens.
///
/// # Errors
/// Returns the error of a write; nothing is written then.
#[tracing::instrument(name = "Seed the Sample Ledger", level = "debug", skip(writer), err)]
pub fn sample(writer: &Writer, now: DateTime<Utc>) -> Result<()> {
    write_sample(writer, now.with_timezone(&Local).date_naive())
}

/// Creates a Ledger file at `path` as `:new` does, from the Sample Ledger's `NewLedger`, then
/// writes the sample rows on top, as [`sample`] adds to a migrated Ledger: the seeded file the
/// Clients' tests open. Opening never seeds (ADR-0054), so this is how a test gets the Sample
/// Ledger in a file.
///
/// # Errors
/// Returns [`crate::Error::LedgerCreation`] if the file can't be created, which leaves nothing at
/// `path`, or the error of the sample's writes.
#[tracing::instrument(name = "Create the Sample Ledger", level = "debug", err)]
pub fn sample_ledger(path: &std::path::Path, today: NaiveDate) -> Result<()> {
    use lib_ledger_creation::LedgerCreation as _;

    crate::LedgerCreationAdapter::new().create_ledger(path, &sample_new_ledger(today))?;
    let (writer, _failures) = Writer::open_ledger(path)?;
    let written = write_sample(&writer, today);
    writer.close()?;
    written
}

/// The moment the Sample Ledger's Units were last priced as of `today`: midday UTC, so the same
/// day always gives the same Weekly Prices, in that day's week whatever the wall clock says.
pub fn sample_priced_at(today: NaiveDate) -> DateTime<Utc> {
    // The moment a Clock set to `today` reads as now, so a test on that day sees fixed ages.
    lib_core::Clock::fixed(today).now()
}

/// `lib-units`' sample as of `today`: the sample Units, priced at [`sample_priced_at`], with aud
/// as the base, and the ids the joined sample's Accounts, Bill Plans and Budgets name.
pub fn sample_units(today: NaiveDate) -> InMemoryPorts {
    InMemoryPorts::sample(sample_priced_at(today))
}

/// What `:new` writes for the Sample Ledger: the sample's aud as the base Unit, and the Personal
/// Institution, Personal spending Budget and default Document Types every new Ledger starts with.
#[expect(
    clippy::expect_used,
    reason = "lib-units' sample holds its own base Unit, which the seed tests cover"
)]
pub fn sample_new_ledger(today: NaiveDate) -> NewLedger {
    let units = sample_units(today);
    let base = units.base_unit.0;
    let base_unit = units
        .units
        .units
        .into_iter()
        .find(|unit| unit.id == base)
        .expect("lib-units' sample holds its base Unit");
    NewLedger::new(base_unit, SAMPLE_PERSONAL)
}

/// The Sample Ledger as of `today`, every domain's rows in the one shape [`sample`],
/// [`sample_ledger`] and `FakeStores::sample` all read: `NewLedger`'s rows with the sample on top.
pub struct SampleLedger {
    pub units: InMemoryPorts,
    pub institutions: Vec<lib_institutions::Institution>,
    pub accounts: Vec<lib_accounts::Account>,
    pub categories: Vec<lib_categories::Category>,
    pub tags: Vec<lib_tags::Tag>,
    pub payees: Vec<lib_payees::Payee>,
    pub transactions: Vec<Transaction>,
    pub bills: BillsSeed,
    pub budgets: Vec<lib_budgets::Budget>,
    pub inventory: lib_inventory::Inventory,
    pub document_types: Vec<lib_documents::types::DocumentTypeRow>,
    pub documents: Vec<lib_documents::Document>,
}

impl SampleLedger {
    pub fn new(today: NaiveDate) -> Self {
        let new = sample_new_ledger(today);
        let mut units = sample_units(today);
        units.base_unit = lib_units::fake::InMemoryBaseUnit(new.base_unit.id);
        replace_or_prepend(&mut units.units.units, new.base_unit, |unit| unit.id);
        let mut institutions = sample_institutions();
        replace_or_prepend(&mut institutions, new.personal, |institution| {
            institution.id
        });
        let documents = sample_documents(today, &institutions);
        Self {
            units,
            institutions,
            accounts: sample_accounts(),
            categories: sample_categories(),
            tags: sample_tags(),
            payees: sample_payees(),
            transactions: sample_transactions(today),
            bills: sample_bills(today),
            budgets: sample_budgets(today),
            inventory: sample_inventory(today),
            document_types: new.document_types,
            documents,
        }
    }
}

/// Puts `row` in place of the row in `rows` with its key, or first if there is none, so a
/// `NewLedger` row stands in for the sample's own and keeps the sample's order.
fn replace_or_prepend<T, K: PartialEq>(rows: &mut Vec<T>, row: T, key: impl Fn(&T) -> K) {
    match rows.iter().position(|existing| key(existing) == key(&row)) {
        Some(index) => rows[index] = row,
        None => rows.insert(0, row),
    }
}

/// Writes the Sample Ledger as of `today` in one transaction. All or nothing, so a seed that
/// fails partway, or a process that dies in it, never leaves a Ledger with a base Unit and only
/// some of the rest (#886). The one place the domains' order lives: each names rows of those
/// before it.
///
/// # Errors
/// Returns the error of a write; nothing is written then.
pub(crate) fn write_sample(writer: &Writer, today: NaiveDate) -> Result<()> {
    write_sample_rows(writer, today, true)
}

/// [`write_sample`], with the Documents left out unless `documents`, for an adapter test that
/// writes its own over the rest of the sample.
pub(crate) fn write_sample_rows(writer: &Writer, today: NaiveDate, documents: bool) -> Result<()> {
    let SampleLedger {
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
        document_types,
        documents: sample_documents,
    } = SampleLedger::new(today);
    let (types, documents) = if documents {
        (document_types, sample_documents)
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

/// `lib-institutions`' sample, the Sample Ledger's Institutions, with the ids its Accounts name.
pub fn sample_institutions() -> Vec<lib_institutions::Institution> {
    lib_institutions::fake::InMemoryInstitutions::sample().institutions
}

/// `lib-accounts`' sample, the Sample Ledger's Accounts, with the ids its Transactions, Bill Plans
/// and Budgets name.
pub fn sample_accounts() -> Vec<lib_accounts::Account> {
    lib_accounts::fake::InMemoryAccounts::sample().accounts
}

/// `lib-categories`' sample, the Sample Ledger's Categories, with the ids its Splits, Bill Plans,
/// Payees and Budgets name.
pub fn sample_categories() -> Vec<lib_categories::Category> {
    lib_categories::fake::InMemoryCategories::sample().categories
}

/// `lib-budgets`' sample as of `today`, the Sample Ledger's Budgets, fed the sample Accounts and
/// Categories as Budgets reads them. Personal spending is `NewLedger`'s, with the sample's Accounts
/// and Limits put on it, as a person would after `:new`.
pub fn sample_budgets(today: NaiveDate) -> Vec<lib_budgets::Budget> {
    let accounts = crate::adapter::accounts::budget_accounts(&sample_accounts());
    let categories = crate::adapter::categories::budget_categories(&sample_categories());
    let personal = sample_new_ledger(today).budget;
    lib_budgets::sample_budgets(&accounts, &categories, today)
        .all()
        .iter()
        .map(|budget| {
            if budget.id == personal.id {
                lib_budgets::Budget {
                    account_ids: budget.account_ids.clone(),
                    limits: budget.limits.clone(),
                    ..personal.clone()
                }
            } else {
                budget.clone()
            }
        })
        .collect()
}

/// `lib-tags`' sample, the Sample Ledger's Tags, with the ids its Splits carry.
pub fn sample_tags() -> Vec<lib_tags::Tag> {
    lib_tags::fake::InMemoryTags::sample().tags
}

/// `lib-inventory`'s sample as of `today`, the Sample Ledger's Inventory, with the Item ids its
/// Documents link.
pub fn sample_inventory(today: NaiveDate) -> lib_inventory::Inventory {
    let lib_inventory::fake::InMemoryInventory { locations, items } =
        lib_inventory::fake::InMemoryInventory::sample(today);
    lib_inventory::Inventory { locations, items }
}

/// `lib-payees`' sample, the Sample Ledger's Payees, with the ids its Splits and Bill Plans name.
pub fn sample_payees() -> Vec<lib_payees::Payee> {
    lib_payees::fake::InMemoryPorts::sample().payees.payees
}

/// The names `lib_transactions::fake`'s sample books by, looked up in the sample collections given.
pub(crate) struct SampleNames<'a> {
    pub(crate) categories: &'a [lib_categories::Category],
    pub(crate) payees: &'a [lib_payees::Payee],
    pub(crate) tags: &'a [lib_tags::Tag],
}

impl lib_transactions::fake::SeedNames for SampleNames<'_> {
    fn category(&self, name: &str) -> Option<CategoryId> {
        lib_categories::find_by_name(self.categories, name)
    }

    fn payee(&self, name: &str) -> Option<PayeeId> {
        lib_payees::find_by_name(self.payees, name)
    }

    fn tag(&self, name: &str) -> Option<TagId> {
        lib_tags::find_by_name(self.tags, name)
    }
}

/// The generated Transaction history over these samples, without the rows the Bills and Documents
/// samples name: newest first, ids assigned oldest-first (a higher id is newer). Each sample Account
/// gets its sample count; accounts that take no Transactions, any account the seed has no profile
/// for, and any Account not in the sample get none.
pub fn sample_transaction_history(
    accounts: &[lib_accounts::Account],
    categories: &[lib_categories::Category],
    payees: &[lib_payees::Payee],
    tags: &[lib_tags::Tag],
    today: NaiveDate,
) -> Vec<Transaction> {
    counted_transactions(
        accounts,
        lib_transactions::fake::sample_transaction_count,
        categories,
        payees,
        tags,
        today,
    )
}

/// The sample Accounts as the generator reads them, with `count` giving each one's number of
/// Transactions.
fn seed_accounts(
    accounts: &[lib_accounts::Account],
    count: impl Fn(AccountId) -> u32,
) -> Vec<lib_transactions::fake::SeedAccount> {
    accounts
        .iter()
        .map(|account| lib_transactions::fake::SeedAccount {
            id: account.id,
            name: account.name.clone(),
            account_type: account.account_type.clone(),
            transaction_count: count(account.id),
        })
        .collect()
}

/// [`sample_transaction_history`] with `count` giving each Account's number of Transactions.
fn counted_transactions(
    accounts: &[lib_accounts::Account],
    count: impl Fn(AccountId) -> u32,
    categories: &[lib_categories::Category],
    payees: &[lib_payees::Payee],
    tags: &[lib_tags::Tag],
    today: NaiveDate,
) -> Vec<Transaction> {
    let names = SampleNames {
        categories,
        payees,
        tags,
    };
    lib_transactions::fake::sample_transactions(&seed_accounts(accounts, count), &names, today)
}

/// `lib_transactions::fake`'s booked sample as of `today` over the per-domain samples: the generated
/// history, then the rows the Bills and Documents samples name, numbered from
/// [`lib_transactions::fake::FIRST_BOOKED`] and each placed newest first, after any already on its
/// date.
pub fn sample_transactions(today: NaiveDate) -> Vec<Transaction> {
    let (categories, payees, tags) = (sample_categories(), sample_payees(), sample_tags());
    let names = SampleNames {
        categories: &categories,
        payees: &payees,
        tags: &tags,
    };
    lib_transactions::fake::booked_sample_transactions(
        &seed_accounts(
            &sample_accounts(),
            lib_transactions::fake::sample_transaction_count,
        ),
        &names,
        today,
    )
}

/// How many of `transactions` are booked to Account `id`: an Account's counted use, for an
/// in-memory Accounts store with no file to count in (#852).
pub fn account_transactions(transactions: &[Transaction], id: AccountId) -> usize {
    transactions
        .iter()
        .filter(|transaction| transaction.account_id == id)
        .count()
}

/// `transactions` flattened as Bills reads them, a [`lib_bills::BillSplit`] per Split in the
/// Transactions' order, as the Transactions store answers `BillTransactions`.
pub fn bill_splits(transactions: &[Transaction]) -> Vec<lib_bills::BillSplit> {
    transactions
        .iter()
        .flat_map(crate::adapter::transactions::bill_splits)
        .collect()
}

/// `lib-bills`' sample Bill Plans and Bill Schedule as of `today`, fed the sample Accounts, Categories
/// and Payees as Bills reads them, whose Paid entries name [`sample_transactions`]' payments.
pub fn sample_bills(today: NaiveDate) -> BillsSeed {
    lib_bills::sample_bills(
        &crate::adapter::accounts::bill_accounts(&sample_accounts()),
        &crate::adapter::categories::bill_categories(&sample_categories()),
        &crate::adapter::payees::bill_payees(&sample_payees()),
        lib_transactions::fake::FIRST_BOOKED,
        today,
    )
    .seed
}

/// The sample rows the Documents' Links name, as the stores answer Documents' read ports, the
/// Accounts' Institutions from `institutions`.
pub fn sample_link_records(
    today: NaiveDate,
    institutions: &[lib_institutions::Institution],
) -> lib_documents::LinkRecords {
    lib_documents::LinkRecords {
        transactions: crate::adapter::transactions::document_transactions(&sample_transactions(
            today,
        )),
        payees: crate::adapter::payees::document_payees(&sample_payees()),
        accounts: crate::adapter::accounts::document_accounts(&sample_accounts(), institutions),
        plans: crate::adapter::bills::document_bill_plans(&sample_bills(today).plans),
        items: crate::adapter::inventory::document_items(&sample_inventory(today).items),
    }
}

/// `lib-documents`' sample Documents as of `today`, fed the sample rows their Links name
/// ([`sample_link_records`]).
pub fn sample_documents(
    today: NaiveDate,
    institutions: &[lib_institutions::Institution],
) -> Vec<lib_documents::Document> {
    lib_documents::sample_documents(&sample_link_records(today, institutions), today).documents
}

/// Fixed so the generated rows are the same on every run, which keeps `fake` seeding idempotent.
const FAKE_RNG_SEED: u64 = 516;

/// Where the generated Units' seed numbers start. The sample's own Units use 1 to 3, so the two never
/// share an id.
const FAKE_FIRST_NUMBER: u32 = 100;

/// `count` generated Units, quoted in the sample's base Unit. The same `count` always gives the same
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

/// Where the generated Transactions' seed numbers start, well clear of the sample's, which number
/// up from 1 to a little over its 700.
const FAKE_FIRST_TRANSACTION: u32 = 100_000;

/// `count` generated Transactions dated within the sample's history before `today`. Each copies the
/// Account and one Split's Category, Payee and Tags from a sample Transaction, so it names rows the
/// Sample Ledger has and keeps the sign its Category takes. The same `count` and `today` always give
/// the same rows, so a second seed upserts over the first rather than adding rows.
pub fn fake_transactions(count: u32, today: NaiveDate) -> Vec<Transaction> {
    let sample = sample_transactions(today);
    let mut rng = StdRng::seed_from_u64(FAKE_RNG_SEED);
    (0..count)
        .filter_map(|index| {
            let model = sample.get(rng.random_range(0..sample.len()))?;
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
                date: today
                    - Duration::days(rng.random_range(0..lib_transactions::fake::HISTORY_DAYS)),
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

/// Where the generated Bill Plans' seed numbers start, clear of the sample's, which number up from 1.
/// Below 4096, so their entries' seed ids stay unique ([`lib_bills::seed_entry_id`]).
const FAKE_FIRST_BILL_PLAN: u32 = 1_000;

/// `count` generated Bill Plans and their generated entries as of `today`, all open. Each copies
/// the Account, Unit, Category and Payee of a sample Bill Plan, so it names rows the Sample Ledger has.
/// The same `count` and `today` always give the same rows, so a second seed upserts over the
/// first rather than adding rows.
pub fn fake_bills(count: u32, today: NaiveDate) -> BillsSeed {
    let sample = sample_bills(today).plans;
    let mut rng = StdRng::seed_from_u64(FAKE_RNG_SEED);
    let plans: Vec<BillPlan> = (0..count)
        .filter_map(|index| {
            let model = sample.get(rng.random_range(0..sample.len()))?;
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
/// Bill Plans on top of the sample, blocking until they land. Run after [`sample`]: the Units are
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

    /// The Institutions are the Sample Ledger's, by id and in order, so the Accounts naming them
    /// name stored rows.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_institutions_twice_over_the_same_rows() {
        use lib_institutions::InstitutionRepository;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored = InstitutionsAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Institutions load");
        assert_eq!(
            stored.institutions(),
            lib_institutions::sample_institutions()
        );
    }

    /// The Personal Institution is the person's once the Ledger exists, so `db-seed` leaves a name
    /// they gave it, while the sample's other Institutions are still put back.
    #[test]
    fn seeding_again_keeps_a_renamed_personal_institution() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        sample(&writer, now).expect("the first seed lands");
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

        sample(&writer, now).expect("the second seed lands");

        let stored = InstitutionsAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Institutions load");
        let name = |id| {
            use lib_institutions::InstitutionRepository;
            stored.institution(id).expect("a sample row").name
        };
        assert_eq!(name(lib_core::InstitutionId::SEED_PERSONAL), "Persönlich");
        assert_eq!(name(lib_core::InstitutionId::SEED_ANZ), "ANZ Banking Group");
    }

    /// The Accounts are the Sample Ledger's, by id and in order, so the Transactions and Bill Plans
    /// naming them name stored rows.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_accounts_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored =
            AccountsAdapter::load(PendingWrites::new(writer.clone())).expect("the Accounts load");
        assert_eq!(
            stored.accounts().accounts(),
            lib_accounts::sample_accounts()
        );
    }

    /// The Tags are the Sample Ledger's, by id, so the Splits written after them carry stored rows.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_tags_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored = TagsAdapter::load(PendingWrites::new(writer.clone())).expect("the Tags load");
        assert_eq!(stored.tags().tags(), lib_tags::sample_tags());
    }

    /// The Inventory is the Sample Ledger's as of the seed's day, by id, so the sample Documents' Links
    /// name stored Items, and it is written last, after the Bills.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_inventory_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored = InventoryAdapter::load(PendingWrites::new(writer.clone()))
            .expect("the Inventory loads");
        let today = now.with_timezone(&Local).date_naive();
        assert_eq!(
            stored.inventory().inventory(),
            lib_inventory::sample_inventory(today)
        );
    }

    /// The Document Types and Documents are the Sample Ledger's as of the seed's day, written last,
    /// over the rows their Links name, so every Link is kept (#963).
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_documents_twice_over_the_same_rows() {
        use lib_documents::DocumentRepository as _;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored =
            DocumentsAdapter::load(PendingWrites::new(writer.clone())).expect("the Documents load");
        let today = now.with_timezone(&Local).date_naive();
        assert_eq!(
            stored.document_types(),
            lib_documents::types::default_types()
        );
        assert_eq!(
            stored.documents(),
            sample_documents(today, &sample_institutions())
        );
    }

    /// Days the Sample Ledger is checked on: a mid-month day, a month's end and a leap day.
    fn sample_days() -> [NaiveDate; 3] {
        [
            today(),
            NaiveDate::from_ymd_opt(2026, 1, 31).expect("a real date"),
            NaiveDate::from_ymd_opt(2028, 2, 29).expect("a real date"),
        ]
    }

    fn sorted_payees(mut payees: Vec<lib_payees::Payee>) -> Vec<lib_payees::Payee> {
        payees.sort_by_key(|payee| payee.id);
        for payee in &mut payees {
            payee.aliases.sort();
        }
        payees
    }

    fn sorted_transactions(mut transactions: Vec<Transaction>) -> Vec<Transaction> {
        transactions.sort_by_key(|transaction| transaction.id);
        transactions
    }

    /// A `seed::sample_ledger` file, opened as a Client opens it, holds `FakeStores::sample`'s rows
    /// in every domain, Unit prices included, so a UI test on the in-memory stores and one on a
    /// file see the same Ledger, and a change to one can't leave the other behind (#1072).
    #[test]
    fn a_sample_ledger_file_holds_fake_stores_samples_rows_in_every_domain() {
        use lib_budgets::BudgetRepository as _;
        use lib_categories::CategoryRepository;
        use lib_documents::DocumentRepository as _;
        use lib_institutions::InstitutionRepository as _;
        use lib_transactions::TransactionRepository as _;

        for today in sample_days() {
            let dir = tempfile::tempdir().expect("a temporary directory");
            let path = dir.path().join("sample.pldb");
            sample_ledger(&path, today).expect("the Sample Ledger is created");
            let file = crate::open_storage(&path).expect("the Sample Ledger opens");
            let memory = crate::FakeStores::sample(today);

            assert_eq!(
                file.units.units().units(),
                memory.units.units.units,
                "Units on {today}"
            );
            assert_eq!(
                file.units.base_unit().base_unit(),
                memory.units.base_unit.0,
                "the base Unit on {today}"
            );
            assert_eq!(
                file.institutions.institutions(),
                memory.institutions.institutions,
                "Institutions on {today}"
            );
            assert_eq!(
                file.accounts.accounts().accounts(),
                memory.accounts.accounts.accounts,
                "Accounts on {today}"
            );
            assert_eq!(
                CategoryRepository::categories(&file.categories),
                CategoryRepository::categories(&memory.categories),
                "Categories on {today}"
            );
            assert_eq!(
                file.tags.tags().tags(),
                memory.tags.tags.tags,
                "Tags on {today}"
            );
            assert_eq!(
                sorted_payees(lib_payees::PayeeRepository::payees(file.payees.payees())),
                sorted_payees(memory.payees.payees.payees),
                "Payees on {today}"
            );
            assert_eq!(
                sorted_transactions(file.transactions.transactions()),
                sorted_transactions(memory.transactions.transactions.clone()),
                "Transactions on {today}"
            );
            assert_eq!(
                lib_bills::contract::snapshot(&file.bills),
                lib_bills::contract::snapshot(&memory.bills),
                "Bills on {today}"
            );
            assert_eq!(
                file.budgets.budgets(),
                memory.budgets.budgets,
                "Budgets on {today}"
            );
            assert_eq!(
                file.inventory.inventory().inventory(),
                lib_inventory::Inventory {
                    locations: memory.inventory.locations,
                    items: memory.inventory.items,
                },
                "the Inventory on {today}"
            );
            assert_eq!(
                file.documents.document_types(),
                memory.documents.types,
                "Document Types on {today}"
            );
            assert_eq!(
                file.documents.documents(),
                memory.documents.documents,
                "Documents on {today}"
            );
        }
    }

    /// The Sample Ledger is what `:new` writes with the sample on top: the Personal Institution,
    /// the base Unit, the Personal spending Budget and the nine default Document Types.
    #[test]
    fn the_sample_ledger_holds_what_new_writes() {
        let new = sample_new_ledger(today());
        let sample = crate::FakeStores::sample(today());

        let personal = sample
            .institutions
            .institutions
            .iter()
            .find(|institution| institution.id == lib_core::InstitutionId::SEED_PERSONAL)
            .expect("the Personal Institution");
        assert_eq!(personal.name, "Personal");
        assert_eq!(personal.usual_unit, None);
        assert_eq!(*personal, new.personal);

        assert_eq!(sample.units.base_unit.0, UnitId::SEED_AUD);
        assert_eq!(new.base_unit.code, "aud");
        assert!(sample.units.units.units.contains(&new.base_unit));

        let spending = sample
            .budgets
            .budgets
            .iter()
            .find(|budget| budget.id == lib_budgets::PERSONAL_SPENDING_ID)
            .expect("Personal spending");
        assert_eq!(spending.name, "Personal spending");
        assert!(spending.is_default);
        assert_eq!(spending.unit, UnitId::SEED_AUD);
        assert_eq!(spending.method, new.budget.method);

        assert_eq!(sample.documents.types.len(), 9);
        assert_eq!(sample.documents.types, new.document_types);
    }

    /// The Sample Ledger for a day is the same whenever it is built, Unit prices included, so no
    /// test depends on the wall clock.
    #[test]
    fn two_fake_stores_samples_for_the_same_day_are_equal_whenever_they_are_built() {
        let first = crate::FakeStores::sample(today());
        std::thread::sleep(std::time::Duration::from_millis(20));
        let second = crate::FakeStores::sample(today());

        assert_eq!(first.units.units.units, second.units.units.units);
        assert_eq!(first.units.base_unit.0, second.units.base_unit.0);
        assert_eq!(
            first.institutions.institutions,
            second.institutions.institutions
        );
        assert_eq!(
            first.accounts.accounts.accounts,
            second.accounts.accounts.accounts
        );
        assert_eq!(first.categories.categories, second.categories.categories);
        assert_eq!(first.tags.tags.tags, second.tags.tags.tags);
        assert_eq!(first.payees.payees.payees, second.payees.payees.payees);
        assert_eq!(first.transactions, second.transactions);
        assert_eq!(first.bills, second.bills);
        assert_eq!(first.budgets, second.budgets);
        assert_eq!(first.inventory, second.inventory);
        assert_eq!(first.documents, second.documents);
    }

    /// The Budgets are the Sample Ledger's as of the seed's day, by id, so Categories 5c finds
    /// Personal spending by `PERSONAL_SPENDING_ID`.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_budgets_twice_over_the_same_rows() {
        use lib_budgets::BudgetRepository as _;

        let (_dir, writer) = temp_writer();
        let now = Utc::now();

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        let stored =
            BudgetsAdapter::load(PendingWrites::new(writer.clone())).expect("the Budgets load");
        let today = now.with_timezone(&Local).date_naive();
        assert_eq!(stored.budgets(), sample_budgets(today));
    }

    /// The Payees are the Sample Ledger's, by id, so the Splits and Bill Plans written after them
    /// name stored rows.
    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_payees_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let mut expected = lib_payees::sample_payees();
        expected.sort_by_key(|payee| payee.id);
        for payee in &mut expected {
            payee.aliases.sort();
        }

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        assert_eq!(stored_payees(&writer), expected);
    }

    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_bills_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let expected = sample_bills(now.with_timezone(&Local).date_naive());

        sample(&writer, now).expect("the first seed lands");
        let first = stored_bills(&writer);
        sample(&writer, now).expect("the second seed lands");

        assert_eq!(first.0.len(), expected.plans.len());
        assert_eq!(first.1.len(), expected.entries.len());
        assert_eq!(stored_bills(&writer), first);
    }

    #[test]
    fn seeding_generated_bills_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        sample(&writer, now).expect("the sample lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = stored_bills(&writer);

        fake(&writer, now, 20).expect("the second generated seed lands");

        let sample = sample_bills(now.with_timezone(&Local).date_naive());
        assert_eq!(first.0.len(), sample.plans.len() + 4);
        assert!(first.1.len() > sample.entries.len());
        assert_eq!(stored_bills(&writer), first);
    }

    #[test]
    fn generated_bills_are_the_same_every_time_and_clear_of_the_sample() {
        let first = fake_bills(4, today());
        let sample_ids: Vec<_> = sample_bills(today())
            .plans
            .iter()
            .map(|plan| plan.id)
            .collect();

        assert_eq!(first.plans.len(), 4);
        assert_eq!(first, fake_bills(4, today()));
        assert!(
            first
                .plans
                .iter()
                .all(|plan| !sample_ids.contains(&plan.id))
        );
        assert!(!first.entries.is_empty());
    }

    #[test]
    fn the_sample_seed_writes_the_sample_ledgers_transactions_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let mut expected = sample_transactions(now.with_timezone(&Local).date_naive());
        expected.sort_by_key(|transaction| transaction.id);

        sample(&writer, now).expect("the first seed lands");
        sample(&writer, now).expect("the second seed lands");

        assert!(
            expected.len() > 700,
            "the Bills and Documents samples add theirs"
        );
        assert_eq!(stored_transactions(&writer), expected);
    }

    #[test]
    fn generated_transactions_are_the_same_every_time_and_clear_of_the_sample() {
        let first = fake_transactions(20, today());
        let sample_ids: Vec<_> = sample_transactions(today())
            .iter()
            .map(|transaction| transaction.id)
            .collect();

        assert_eq!(first.len(), 20);
        assert_eq!(first, fake_transactions(20, today()));
        assert!(
            first
                .iter()
                .all(|transaction| !sample_ids.contains(&transaction.id))
        );
    }

    #[test]
    fn seeding_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        sample(&writer, now).expect("the first seed lands");
        let first = UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads");
        let units = first.units().units();
        let base = first.base_unit().base_unit();

        sample(&writer, now).expect("the second seed lands");
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
        sample(&writer, now).expect("the first seed lands");
        let pending = PendingWrites::new(writer.clone());
        let mut adapter = UnitsAdapter::load(pending.clone()).expect("the seed loads");
        assert_eq!(adapter.base_unit().base_unit(), UnitId::SEED_AUD);
        let changed = adapter
            .units()
            .units()
            .into_iter()
            .map(|unit| unit.id)
            .find(|id| *id != UnitId::SEED_AUD)
            .expect("the sample has a second Unit");
        let usage = lib_units::fake::InMemoryUnitUsage::default();
        adapter.split_mut(&usage).base_unit.set_base_unit(changed);
        pending.commit();

        sample(&writer, now).expect("the second seed lands");
        let reseeded =
            UnitsAdapter::load(PendingWrites::new(writer.clone())).expect("the seed loads again");

        assert_eq!(reseeded.base_unit().base_unit(), changed);
    }

    #[test]
    fn generated_units_are_the_same_every_time_and_clear_of_the_sample() {
        let now = Utc::now();
        let first = fake_units(20, now);
        let second = fake_units(20, now);
        let sample_ids: Vec<_> = sample_units(today())
            .units
            .units
            .iter()
            .map(|unit| unit.id)
            .collect();

        assert_eq!(first, second);
        assert!(first.iter().all(|unit| !sample_ids.contains(&unit.id)));
        let mut codes: Vec<_> = first.iter().map(|unit| unit.code.clone()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), first.len());
    }

    #[test]
    fn seeding_generated_units_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        sample(&writer, now).expect("the sample lands");
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
            sample(&writer, now).expect("the sample lands");
            fake(&writer, now, 20).expect("the generated seed lands");
            let units = UnitsAdapter::load(PendingWrites::new(writer.clone()))
                .expect("the seed loads")
                .units()
                .units();
            // The sample's Units are priced from the seed's local day, the generated ones from
            // `now`, which can fall in the week either side of it.
            let sample_week =
                lib_units::WeeklyPrice::week_of(now.with_timezone(&Local).date_naive());
            let generated_week = lib_units::WeeklyPrice::week_of(now.date_naive());
            let sample_ids: Vec<_> = sample_units(now.with_timezone(&Local).date_naive())
                .units
                .units
                .iter()
                .map(|unit| unit.id)
                .collect();

            let priced: Vec<_> = units
                .iter()
                .filter(|unit| !unit.prices.is_empty())
                .collect();

            assert!(
                priced.iter().any(|unit| unit.id == UnitId::SEED_BTC),
                "the sample's btc on {now}"
            );
            assert!(priced.len() > 1, "and some generated Units on {now}");
            for unit in priced {
                let current = if sample_ids.contains(&unit.id) {
                    sample_week
                } else {
                    generated_week
                };
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
        sample(&writer, now).expect("the sample lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = stored_transactions(&writer);

        fake(&writer, now, 20).expect("the second generated seed lands");

        let sample = sample_transactions(now.with_timezone(&Local).date_naive());
        assert_eq!(first.len(), sample.len() + 20);
        assert_eq!(stored_transactions(&writer), first);
    }
}
