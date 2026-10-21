//! The dev seed: each persisted domain's stub seed, written through its adapter. A module's stub
//! seed is the one seed for its domain, so a change to a persisted entity updates that seed and
//! this follows. Ids match `lib-ledger`'s stub, which the domains not yet persisted still read.
//!
//! Every write is an upsert, so seeding twice leaves the same rows.

use chrono::{DateTime, Utc};
use lib_units::fake::InMemoryPorts;

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
}
