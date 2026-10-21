//! The dev seed: each persisted domain's stub seed, written through its adapter. A module's stub
//! seed is the one seed for its domain, so a change to a persisted entity updates that seed and
//! this follows. Ids match `lib-ledger`'s stub, which the domains not yet persisted still read.
//!
//! Every write is an upsert, so seeding twice leaves the same rows. The base Unit is the exception:
//! it is written only when the Ledger has none, so a base the person changed survives a re-seed.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use fake::faker::company::en::CompanyName;
use fake::faker::currency::en::CurrencyCode;
use fake::rand::rngs::StdRng;
use fake::rand::{Rng, SeedableRng};
use fake::{Fake, Faker};
use lib_core::{Money, UnitId, UnitKind};
use lib_units::fake::InMemoryPorts;
use lib_units::{LastPrice, PriceLink, UnitRow};

use crate::Result;
use crate::adapter::units::UnitsAdapter;
use crate::writer::Writer;

/// Writes every persisted domain's stub seed, blocking until it lands. Today that is Units, Price
/// Sources and the base Unit; each later domain adds its seed as it migrates.
///
/// # Errors
/// Returns the error of the write; nothing is written then.
#[tracing::instrument(name = "Seed the stub", level = "debug", skip(writer), err)]
pub fn stub(writer: &Writer, now: DateTime<Utc>) -> Result<()> {
    let units = InMemoryPorts::stub(now);
    UnitsAdapter::seed(
        writer,
        units.units.units,
        units.price_sources.records,
        units.base_unit.0,
    )
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

/// Writes `count` generated Units on top of the stub seed, blocking until they land. Run after
/// [`stub`]: they are quoted in its base Unit.
///
/// # Errors
/// Returns the error of the write; nothing is written then.
#[tracing::instrument(name = "Seed the generated Units", level = "debug", skip(writer), err)]
pub fn fake(writer: &Writer, now: DateTime<Utc>, count: u32) -> Result<()> {
    UnitsAdapter::seed(writer, fake_units(count, now), Vec::new(), UnitId::SEED_AUD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_writer() -> (tempfile::TempDir, Writer) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let (writer, _failures) =
            Writer::open_ledger(&dir.path().join("ledger.db")).expect("a new Ledger opens");
        (dir, writer)
    }

    #[test]
    fn seeding_twice_leaves_the_same_rows() {
        let (_dir, writer) = temp_writer();
        let now = Utc::now();
        stub(&writer, now).expect("the first seed lands");
        let first = UnitsAdapter::load(writer.clone()).expect("the seed loads");
        let units = first.units().units();
        let base = first.base_unit().base_unit();

        stub(&writer, now).expect("the second seed lands");
        let second = UnitsAdapter::load(writer.clone()).expect("the seed loads again");

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
        let mut adapter = UnitsAdapter::load(writer.clone()).expect("the seed loads");
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
        adapter.commit();

        stub(&writer, now).expect("the second seed lands");
        let reseeded = UnitsAdapter::load(writer.clone()).expect("the seed loads again");

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
        let first = UnitsAdapter::load(writer.clone()).expect("the seed loads");
        let units = first.units().units();

        fake(&writer, now, 20).expect("the second generated seed lands");
        let second = UnitsAdapter::load(writer.clone()).expect("the seed loads again");

        assert_eq!(units.len(), 3 + 20);
        assert_eq!(second.units().units(), units);
    }
}
