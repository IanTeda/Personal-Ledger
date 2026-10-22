//! The dev seed: each persisted domain's stub seed, written through its adapter. A module's stub
//! seed is the one seed for its domain, so a change to a persisted entity updates that seed and
//! this follows. Ids match `lib-ledger`'s stub, which the domains not yet persisted still read.
//!
//! The Transactions are the stub Ledger's rather than `lib_transactions::fake`'s alone: the Bills
//! and Documents stubs add the Transactions their Paid entries and Links name (#774). They come
//! from `lib-ledger` until those domains persist their own seeds.
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
use lib_core::{Money, TransactionId, TransactionStatus, UnitId, UnitKind};
use lib_transactions::{Split, Transaction};
use lib_units::fake::InMemoryPorts;
use lib_units::{LastPrice, PriceLink, UnitRow};

use crate::Result;
use crate::adapter::transactions::TransactionsAdapter;
use crate::adapter::units::UnitsAdapter;
use crate::writer::Writer;

/// Writes every persisted domain's stub seed, blocking until it lands: Units, Price Sources and
/// the base Unit, then the Transactions dated from `now`'s local day. Each later domain adds its
/// seed as it migrates.
///
/// # Errors
/// Returns the error of a write; nothing of that domain's is written then.
#[tracing::instrument(name = "Seed the stub", level = "debug", skip(writer), err)]
pub fn stub(writer: &Writer, now: DateTime<Utc>) -> Result<()> {
    UnitsAdapter::seed_from(writer, InMemoryPorts::stub(now))?;
    TransactionsAdapter::seed(
        writer,
        stub_ledger_transactions(now.with_timezone(&Local).date_naive()),
    )
}

/// The stub Ledger's Transactions as of `today`: `lib_transactions::fake`'s, with those the Bills
/// and Documents stubs add.
pub fn stub_ledger_transactions(today: NaiveDate) -> Vec<Transaction> {
    lib_ledger::Ledger::stub_on(today).transactions().to_vec()
}

/// Fixed so the generated rows are the same on every run, which keeps `fake` seeding idempotent.
const FAKE_RNG_SEED: u64 = 516;

/// Where the generated Units' seed numbers start. The stub's own Units use 1 to 3, so the two never
/// share an id.
const FAKE_FIRST_NUMBER: u32 = 100;

/// `count` generated Units, quoted in the stub's base Unit. The same `count` always gives the same
/// ids, codes and names, so a second seed upserts over the first rather than adding rows. Only the
/// prices' fetch times follow `now`.
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
            let last_price = (kind != UnitKind::Currency).then(|| LastPrice {
                price: Money(BigDecimal::new(rng.random_range(1..=2_000_000).into(), 2)),
                fetched_at: now - Duration::minutes(rng.random_range(1..=1_440)),
            });
            UnitRow {
                id: UnitId::seed(number),
                code,
                name: CompanyName().fake_with_rng(&mut rng),
                kind,
                link: PriceLink::manual(UnitId::SEED_AUD),
                last_price,
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
    let stub = stub_ledger_transactions(today);
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

/// Writes `count` generated Units and `count` generated Transactions on top of the stub seed,
/// blocking until they land. Run after [`stub`]: the Units are quoted in its base Unit, and the
/// Transactions name its Accounts, Categories, Payees and Tags.
///
/// # Errors
/// Returns the error of a write; nothing of that domain's is written then.
#[tracing::instrument(name = "Seed the generated rows", level = "debug", skip(writer), err)]
pub fn fake(writer: &Writer, now: DateTime<Utc>, count: u32) -> Result<()> {
    UnitsAdapter::seed(writer, fake_units(count, now), Vec::new(), UnitId::SEED_AUD)?;
    TransactionsAdapter::seed(
        writer,
        fake_transactions(count, now.with_timezone(&Local).date_naive()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::{Commit, PendingWrites};

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

    #[test]
    fn the_stub_seed_writes_the_stub_ledgers_transactions_twice_over_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        let mut expected = stub_ledger_transactions(now.with_timezone(&Local).date_naive());
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
        let stub_ids: Vec<_> = stub_ledger_transactions(today())
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

    #[test]
    fn seeding_generated_transactions_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the stub lands");
        fake(&writer, now, 20).expect("the first generated seed lands");
        let first = stored_transactions(&writer);

        fake(&writer, now, 20).expect("the second generated seed lands");

        let stub = stub_ledger_transactions(now.with_timezone(&Local).date_naive());
        assert_eq!(first.len(), stub.len() + 20);
        assert_eq!(stored_transactions(&writer), first);
    }
}
