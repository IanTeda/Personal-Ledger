//! A Client's Ledger file, opened at start-up: the shared writer on the `[Personal-Ledger] file`,
//! the Client migrations, then the Units adapter on the pending batch every adapter shares. The
//! Desktop and the TUI both open it here, so they read and write the same rows (ADR-0046).

use std::path::Path;
use std::sync::mpsc;

use chrono::Utc;
use lib_units::fake::InMemoryPorts;

use crate::{Error, PendingWrites, Result, UnitsAdapter, WriteFailure, Writer};

/// The Ledger file as start-up opens it: the Units adapter, the pending batch the bin commits
/// after each Interactor call, and the writer every adapter saves through, with its failure
/// reports.
pub struct Storage {
    pub units: UnitsAdapter,
    pub pending: PendingWrites,
    /// Flushed when the Client quits, so the last writes land before the thread dies.
    pub writer: Writer,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// Opens the Ledger at `path`, creating and migrating it if need be, and loads the Units, blocking.
/// A Ledger with no base Unit is new, so it is seeded first with the stub's Units, Price Sources
/// and base Unit: the stub Accounts, Bill Plans and Budgets still name them (#746's first-launch
/// seed, removed once every domain persists).
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path) -> Result<Storage> {
    let (writer, failures) = Writer::open_ledger(path)?;
    let pending = PendingWrites::new(writer.clone());
    let units = match UnitsAdapter::load(pending.clone()) {
        Err(Error::NotFound(_)) => {
            let seed = InMemoryPorts::stub(Utc::now());
            UnitsAdapter::seed_from(&writer, seed)?;
            tracing::info!(path = %path.display(), "Seeded a new Ledger with the stub's Units");
            UnitsAdapter::load(pending.clone())?
        }
        loaded => loaded?,
    };
    Ok(Storage {
        units,
        pending,
        writer,
        failures,
    })
}
