//! A Client's Ledger file, opened at start-up: the shared writer on the `[Personal-Ledger] file`,
//! the Client migrations, then the Units and Transactions adapters on the pending batch every
//! adapter shares. The
//! Desktop and the TUI both open it here, so they read and write the same rows (ADR-0046).

use std::path::Path;
use std::sync::mpsc;

use chrono::{NaiveDate, Utc};
use lib_units::fake::InMemoryPorts;

use crate::{
    Error, PendingWrites, Result, TransactionsAdapter, UnitsAdapter, WriteFailure, Writer, seed,
};

/// The Ledger file as start-up opens it: each adapter, the pending batch the bin commits after
/// each Interactor call, and the writer every adapter saves through, with its failure reports.
pub struct Storage {
    pub units: UnitsAdapter,
    pub transactions: TransactionsAdapter,
    pub pending: PendingWrites,
    /// Flushed when the Client quits, so the last writes land before the thread dies.
    pub writer: Writer,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// Opens the Ledger at `path`, creating and migrating it if need be, and loads every adapter,
/// blocking. A Ledger with no base Unit is new, so it is seeded first with the stub's Units, Price
/// Sources and base Unit, which the stub Accounts, Bill Plans and Budgets still name (#746), and
/// the stub Ledger's Transactions as of `today`, which the stub Bill Schedule and Documents still
/// name (#774). The first-launch seed is removed once every domain persists.
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
            TransactionsAdapter::seed(&writer, seed::stub_ledger_transactions(today))?;
            tracing::info!(path = %path.display(), "Seeded a new Ledger with the stub");
            UnitsAdapter::load(pending.clone())?
        }
        loaded => loaded?,
    };
    let transactions = TransactionsAdapter::load(pending.clone())?;
    Ok(Storage {
        units,
        transactions,
        pending,
        writer,
        failures,
    })
}

#[cfg(test)]
mod tests {
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
}
