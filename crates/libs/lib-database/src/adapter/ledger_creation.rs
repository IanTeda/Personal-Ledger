//! The Ledger Creation adapter: `lib-ledger-creation`'s atomic port over a path (ADR-0054). Unlike
//! the other cross-domain adapters it holds no stores, since none exist before the file does: it
//! claims the name, migrates the file through its own [`Writer`] and writes every starting row in
//! one transaction, then closes the file. Anything that fails after the name is claimed removes the
//! file and its journals again, so a Ledger file is either complete or absent.

use std::path::Path;

use lib_ledger_creation::{CreationFailure, LedgerCreation, NewLedger};

use crate::adapter::budgets::BudgetsAdapter;
use crate::adapter::documents::DocumentsAdapter;
use crate::adapter::institutions::InstitutionsAdapter;
use crate::adapter::units::UnitsAdapter;
use crate::{Error, Result, Writer};

/// The creation port over the file system.
#[derive(Debug, Default)]
pub struct LedgerCreationAdapter {
    /// Fails every create just before its transaction commits, for the contract's failed create.
    fail_before_commit: bool,
}

impl LedgerCreationAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// An adapter whose every create fails once the file is claimed, migrated and its rows sent,
    /// so a test sees what a failure there leaves behind.
    #[doc(hidden)]
    pub fn failing_for_test() -> Self {
        Self {
            fail_before_commit: true,
        }
    }
}

impl LedgerCreation for LedgerCreationAdapter {
    fn create_ledger(
        &mut self,
        path: &Path,
        ledger: &NewLedger,
    ) -> std::result::Result<(), CreationFailure> {
        // A stale journal beside the name would replay into the new file, so it counts as taken.
        if lib_shell::explorer::ledger_files(path)
            .iter()
            .skip(1)
            .any(|journal| journal.exists())
        {
            tracing::error!(path = %path.display(), "A journal is already beside the new Ledger file");
            return Err(CreationFailure::Exists);
        }
        // `create_new` claims the name atomically, so a file made after the explorer checked the
        // name is refused rather than overwritten.
        if let Err(error) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            tracing::error!(%error, path = %path.display(), "Couldn't claim the new Ledger file");
            return Err(if error.kind() == std::io::ErrorKind::AlreadyExists {
                CreationFailure::Exists
            } else {
                CreationFailure::Unwritable
            });
        }
        match write(path, ledger.clone(), self.fail_before_commit) {
            Ok(()) => {
                tracing::debug!(path = %path.display(), "Wrote a new Ledger's starting rows");
                Ok(())
            }
            Err(error) => {
                tracing::error!(%error, path = %path.display(), "Failed to create the Ledger file");
                remove_ledger_files(path);
                Err(CreationFailure::Unwritable)
            }
        }
    }
}

/// Migrates the claimed file at `path` and writes `ledger`'s rows in one transaction, closing the
/// file whether or not they land.
#[tracing::instrument(name = "Write a new Ledger", level = "debug", skip_all, err)]
fn write(path: &Path, ledger: NewLedger, fail_before_commit: bool) -> Result<()> {
    let (writer, _failures) = Writer::open_ledger(path)?;
    let written = writer.run(move |pool| async move {
        let NewLedger {
            personal,
            base_unit,
            budget,
            document_types,
        } = ledger;
        let mut conn = pool.begin().await?;
        let base = base_unit.id;
        UnitsAdapter::seed_in(&mut conn, &[base_unit], &[], base).await?;
        InstitutionsAdapter::seed_in(&mut conn, &[personal]).await?;
        BudgetsAdapter::seed_in(&mut conn, &[budget]).await?;
        DocumentsAdapter::seed_in(&mut conn, &document_types, &[]).await?;
        if fail_before_commit {
            return Err(Error::Generic("the test refuses the commit".into()));
        }
        conn.commit().await?;
        Ok(())
    });
    // Released before returning, so a failed create can remove the file with nothing writing it.
    writer.close()?;
    written
}

/// Removes the Ledger file a failed create claimed, with the journal files SQLite keeps beside it.
/// A Client calls it too, for a Ledger it created but then couldn't open, so a create that ends
/// in an error leaves no file whichever step failed.
pub fn remove_ledger_files(path: &Path) {
    for file in lib_shell::explorer::ledger_files(path) {
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::error!(%error, path = %file.display(), "Couldn't remove a failed Ledger file");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lib_ledger_creation::contract::{self, CreationFactory, Stored};

    use super::*;
    use crate::{Stores, open_storage};

    /// The adapter on a fresh folder, read back through `open_storage`, as a Client opens a Ledger.
    struct AdapterFactory {
        folder: tempfile::TempDir,
        next: usize,
    }

    impl AdapterFactory {
        fn new() -> Self {
            Self {
                folder: tempfile::tempdir().expect("a temporary directory"),
                next: 0,
            }
        }
    }

    impl CreationFactory for AdapterFactory {
        type Port = LedgerCreationAdapter;

        fn port(&mut self) -> LedgerCreationAdapter {
            LedgerCreationAdapter::new()
        }

        fn free_path(&mut self) -> PathBuf {
            self.next += 1;
            self.folder.path().join(format!("{}.pldb", self.next))
        }

        fn occupy(&mut self, _port: &mut LedgerCreationAdapter, path: &Path) {
            std::fs::write(path, "someone's file").expect("a writable folder");
        }

        fn still_occupied(&mut self, _port: &LedgerCreationAdapter, path: &Path) -> bool {
            std::fs::read_to_string(path).is_ok_and(|text| text == "someone's file")
        }

        fn break_creates(&mut self, port: &mut LedgerCreationAdapter) {
            *port = LedgerCreationAdapter::failing_for_test();
        }

        fn stored(&mut self, _port: &LedgerCreationAdapter, path: &Path) -> Option<Stored> {
            if lib_shell::explorer::ledger_files(path)
                .iter()
                .all(|file| !file.exists())
            {
                return None;
            }
            let storage = open_storage(path).expect("the Ledger opens");
            storage.writer.flush().expect("the writer is running");
            let stores = Stores::from(storage);
            let other_rows = stores.accounts.accounts().accounts().len()
                + stores.categories.categories().categories().len()
                + stores.tags.tags().tags().len()
                + stores.payees.payees().payees().len()
                + stores.transactions.transactions().transactions().len()
                + stores.bills.plans().bill_plans().len()
                + stores.inventory.inventory().inventory().locations.len()
                + stores.documents.documents().documents().len();
            Some(Stored {
                units: stores.units.units().units(),
                base_unit: stores.units.base_unit().base_unit(),
                institutions: stores.institutions.institutions().institutions(),
                budgets: stores.budgets.budgets().budgets(),
                document_types: stores.documents.documents().document_types(),
                other_rows,
            })
        }
    }

    #[test]
    fn the_adapter_stores_the_starting_rows() {
        contract::a_create_stores_the_starting_rows_and_nothing_else(&mut AdapterFactory::new());
    }

    #[test]
    fn the_adapter_never_overwrites_an_existing_file() {
        contract::an_existing_file_is_never_overwritten(&mut AdapterFactory::new());
    }

    #[test]
    fn the_adapter_never_creates_over_a_ledger() {
        contract::a_ledger_is_never_created_over(&mut AdapterFactory::new());
    }

    #[test]
    fn the_adapter_leaves_no_file_for_a_failed_create() {
        contract::a_failed_create_leaves_no_file(&mut AdapterFactory::new());
    }

    #[test]
    fn a_name_whose_journal_is_still_there_is_taken_and_the_journal_left_alone() {
        let folder = tempfile::tempdir().expect("a temporary directory");
        let path = folder.path().join("teda.pldb");
        let journal = folder.path().join("teda.pldb-wal");
        std::fs::write(&journal, "an old Ledger's journal").expect("a writable folder");

        assert_eq!(
            LedgerCreationAdapter::new().create_ledger(&path, &contract::usd_ledger()),
            Err(CreationFailure::Exists)
        );

        assert!(!path.exists());
        assert!(journal.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_that_cannot_be_written_is_unwritable_and_leaves_nothing() {
        use std::os::unix::fs::PermissionsExt;

        let folder = tempfile::tempdir().expect("a temporary directory");
        let locked = folder.path().join("locked");
        std::fs::create_dir(&locked).expect("a folder");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555))
            .expect("a read-only folder");

        let created = LedgerCreationAdapter::new()
            .create_ledger(&locked.join("teda.pldb"), &contract::usd_ledger());
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
            .expect("the folder writable again, for the clean-up");

        assert_eq!(created, Err(CreationFailure::Unwritable));
        assert_eq!(std::fs::read_dir(&locked).expect("the folder").count(), 0);
    }
}
