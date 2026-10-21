//! The TUI's Ledger file, opened at start-up the way the Desktop opens it: the shared writer on
//! the `[Personal-Ledger] file`, the Client migrations, then the Units adapter, so both Clients
//! read and write the same rows (ADR-0046).

use std::path::Path;
use std::sync::mpsc;

use chrono::Utc;
use lib_database::{UnitsAdapter, WriteFailure, Writer};
use lib_units::fake::InMemoryPorts;

/// The Ledger file as start-up opens it: the Units adapter, and the writer every adapter saves
/// through, with its failure reports.
pub struct Storage {
    pub units: UnitsAdapter,
    /// Flushed when the TUI quits, so the last writes land before the thread dies.
    pub writer: Writer,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// Opens the Ledger at `path`, creating and migrating it if need be, and loads the Units, blocking.
/// A Ledger with no base Unit is new, so it is seeded first with the stub's Units, Price Sources
/// and base Unit, as the Desktop seeds it: the stub Accounts, Bill Plans and Budgets still name
/// them (#746's first-launch seed, removed once every domain persists).
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path) -> lib_database::Result<Storage> {
    let (writer, failures) = Writer::open_ledger(path)?;
    let units = match UnitsAdapter::load(writer.clone()) {
        Err(lib_database::Error::NotFound(_)) => {
            let seed = InMemoryPorts::stub(Utc::now());
            UnitsAdapter::seed(
                &writer,
                seed.units.units,
                seed.price_sources.records,
                seed.base_unit.0,
            )?;
            tracing::info!(path = %path.display(), "Seeded a new Ledger with the stub's Units");
            UnitsAdapter::load(writer.clone())?
        }
        loaded => loaded?,
    };
    Ok(Storage {
        units,
        writer,
        failures,
    })
}
