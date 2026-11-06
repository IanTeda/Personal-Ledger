//! The contract every creation port keeps (ADR-0054), run against both the in-memory port here and
//! the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a create stores
//! the starting rows and nothing else, a file already at the path is never overwritten, and a
//! create that fails leaves no file.
//!
//! Each check is a function generic over a [`CreationFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh folder per test).

use std::path::{Path, PathBuf};

use lib_budgets::Budget;
use lib_core::UnitId;
use lib_documents::types::DocumentTypeRow;
use lib_institutions::Institution;
use lib_units::{Precision, PriceLink, UnitKind, UnitRow};

use crate::fake::InMemoryLedgerCreation;
use crate::protocols::{CreationFailure, LedgerCreation};
use crate::starting::NewLedger;

/// What a Ledger file holds, read back as opening it would.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub units: Vec<UnitRow>,
    pub base_unit: UnitId,
    pub institutions: Vec<Institution>,
    pub budgets: Vec<Budget>,
    pub document_types: Vec<DocumentTypeRow>,
    /// Every Account, Category, Tag, Payee, Transaction, Bill Plan, Inventory Location and
    /// Document, none of which a new Ledger has.
    pub other_rows: usize,
}

impl Stored {
    fn of(ledger: &NewLedger) -> Self {
        Self {
            units: vec![ledger.base_unit.clone()],
            base_unit: ledger.base_unit.id,
            institutions: vec![ledger.personal.clone()],
            budgets: vec![ledger.budget.clone()],
            document_types: ledger.document_types.clone(),
            other_rows: 0,
        }
    }
}

/// Builds the port a contract check runs on, and the paths it creates at.
pub trait CreationFactory {
    type Port: LedgerCreation;

    fn port(&mut self) -> Self::Port;

    /// A path nothing is at yet, in a folder the port can write.
    fn free_path(&mut self) -> PathBuf;

    /// Puts something other than a Ledger at `path`, as another app's file would be.
    fn occupy(&mut self, port: &mut Self::Port, path: &Path);

    /// Whether what [`Self::occupy`] put at `path` is still there as it was.
    fn still_occupied(&mut self, port: &Self::Port, path: &Path) -> bool;

    /// Makes every create on `port` from now on fail after it has claimed the name.
    fn break_creates(&mut self, port: &mut Self::Port);

    /// What the Ledger file at `path` holds, or `None` when nothing is at `path`.
    fn stored(&mut self, port: &Self::Port, path: &Path) -> Option<Stored>;
}

/// The in-memory port is its own storage.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    next: usize,
}

impl CreationFactory for InMemoryFactory {
    type Port = InMemoryLedgerCreation;

    fn port(&mut self) -> InMemoryLedgerCreation {
        InMemoryLedgerCreation::default()
    }

    fn free_path(&mut self) -> PathBuf {
        self.next += 1;
        PathBuf::from(format!("/ledgers/{}.pldb", self.next))
    }

    fn occupy(&mut self, port: &mut InMemoryLedgerCreation, path: &Path) {
        port.occupied.insert(path.to_path_buf());
    }

    fn still_occupied(&mut self, port: &InMemoryLedgerCreation, path: &Path) -> bool {
        port.occupied.contains(path) && !port.ledgers.contains_key(path)
    }

    fn break_creates(&mut self, port: &mut InMemoryLedgerCreation) {
        port.broken = true;
    }

    fn stored(&mut self, port: &InMemoryLedgerCreation, path: &Path) -> Option<Stored> {
        port.ledgers.get(path).map(Stored::of)
    }
}

/// A new Ledger on a US Dollar base Unit, as en-US prefills it.
pub fn usd_ledger() -> NewLedger {
    let id = UnitId::new();
    let unit = UnitRow {
        id,
        ..UnitRow::new(
            "usd".to_string(),
            "US Dollar".to_string(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink::manual(id),
        )
    };
    NewLedger::new(unit, "Personal")
}

/// A create stores the Personal Institution, the base Unit, Personal spending in it and the
/// default Document Types, and nothing else.
pub fn a_create_stores_the_starting_rows_and_nothing_else<F: CreationFactory>(factory: &mut F) {
    let mut port = factory.port();
    let path = factory.free_path();
    let ledger = usd_ledger();

    port.create_ledger(&path, &ledger)
        .expect("nothing is at the path");

    assert_eq!(factory.stored(&port, &path), Some(Stored::of(&ledger)));
}

/// A create at a path something is already at is refused and leaves it as it was.
pub fn an_existing_file_is_never_overwritten<F: CreationFactory>(factory: &mut F) {
    let mut port = factory.port();
    let path = factory.free_path();
    factory.occupy(&mut port, &path);

    assert_eq!(
        port.create_ledger(&path, &usd_ledger()),
        Err(CreationFailure::Exists)
    );

    assert!(
        factory.still_occupied(&port, &path),
        "the file is left alone"
    );
}

/// A second create at a Ledger's path is refused and the first Ledger stands.
pub fn a_ledger_is_never_created_over<F: CreationFactory>(factory: &mut F) {
    let mut port = factory.port();
    let path = factory.free_path();
    let first = usd_ledger();
    port.create_ledger(&path, &first)
        .expect("nothing is at the path");

    assert_eq!(
        port.create_ledger(&path, &usd_ledger()),
        Err(CreationFailure::Exists)
    );

    assert_eq!(factory.stored(&port, &path), Some(Stored::of(&first)));
}

/// A create that fails once it has claimed the name leaves nothing at the path.
pub fn a_failed_create_leaves_no_file<F: CreationFactory>(factory: &mut F) {
    let mut port = factory.port();
    let path = factory.free_path();
    factory.break_creates(&mut port);

    assert_eq!(
        port.create_ledger(&path, &usd_ledger()),
        Err(CreationFailure::Unwritable)
    );

    assert_eq!(factory.stored(&port, &path), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_stores_the_starting_rows() {
        a_create_stores_the_starting_rows_and_nothing_else(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_never_overwrites_an_existing_file() {
        an_existing_file_is_never_overwritten(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_never_creates_over_a_ledger() {
        a_ledger_is_never_created_over(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_leaves_no_file_for_a_failed_create() {
        a_failed_create_leaves_no_file(&mut InMemoryFactory::default());
    }
}
