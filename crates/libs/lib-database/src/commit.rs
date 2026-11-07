//! Where one Interactor call's writes wait, from every adapter, until the bin commits them
//! (ADR-0046). Each adapter holds a clone of the one [`PendingWrites`] and adds each port write to
//! it, in the order made. The ports can't tell where an Interactor call ends, so the bin calls
//! [`Commit::commit`] once after each Presenter `handle`, and a call that writes two domains, such
//! as a Transaction and the Bill Schedule entry it pays, lands as one transaction.
//!
//! The commit also says what the input changed ([`Committed`]): each store [`Stores`](crate::Stores)
//! holds notes its [`Domain`] in the commit's [`Written`] on every mutable access, so a Client
//! announces from the commit rather than counting its own writes.

use std::cell::RefCell;
use std::rc::Rc;

use crate::writer::{Batch, Change, Domain, Writer};

/// Ends one Interactor call's writes. What the bins hold, so they commit the same way on the
/// Ledger file's adapters and on a test's in-memory ports.
pub trait Commit {
    /// Sends every write made since the last commit as one transaction. Never waits: a failure is
    /// reported on the writer's failure channel. Returns what the input changed: the domains
    /// written, and each domain a cascade of the writes changes in the file, whose adapter the
    /// caller reloads after (ADR-0046's 2026-11-02 amendment): a reload goes through the writer, so
    /// it is queued behind this commit and sees the cascade.
    fn commit(&self) -> Committed;
}

/// What one commit changed, each domain once, in the order first written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Committed {
    /// Every domain a store was mutably reached for since the last commit, and every domain a
    /// queued write names.
    pub wrote: Vec<Domain>,
    /// Every domain the file's `ON DELETE CASCADE` changes behind memory's back, for the caller to
    /// reload.
    pub cascaded: Vec<Domain>,
}

impl Committed {
    /// Whether the input changed nothing, so nothing needs announcing.
    pub fn is_empty(&self) -> bool {
        self.wrote.is_empty() && self.cascaded.is_empty()
    }
}

/// The domains written since the last commit, noted by each store's recorder and drained by the
/// commit. Single-threaded, as the ports are; a clone shares the same set.
#[derive(Debug, Clone, Default)]
pub struct Written(Rc<RefCell<Vec<Domain>>>);

impl Written {
    /// Notes that `domain` was written, once, behind those already noted.
    pub fn note(&self, domain: Domain) {
        let mut written = self.0.borrow_mut();
        if !written.contains(&domain) {
            written.push(domain);
        }
    }

    /// The domains noted since the last take, leaving none.
    pub fn take(&self) -> Vec<Domain> {
        std::mem::take(&mut *self.0.borrow_mut())
    }
}

/// The writes every adapter on one [`Writer`] has made since the last commit. Single-threaded, as
/// the ports are; a clone shares the same writes.
#[derive(Clone)]
pub struct PendingWrites {
    changes: Rc<RefCell<Vec<Box<dyn Change>>>>,
    written: Written,
    writer: Writer,
}

impl PendingWrites {
    pub fn new(writer: Writer) -> Self {
        Self {
            changes: Rc::default(),
            written: Written::default(),
            writer,
        }
    }

    /// The domains noted as written since the last commit, which the stores' recorders share.
    pub fn written(&self) -> &Written {
        &self.written
    }

    /// The writer the writes go to, which an adapter also loads through.
    pub fn writer(&self) -> &Writer {
        &self.writer
    }

    /// Adds one port write, behind those already made.
    pub fn push(&self, change: impl Change) {
        self.changes.borrow_mut().push(Box::new(change));
    }

    /// Drops `domain`'s writes not yet committed, for its adapter's reload: memory is read back
    /// from the file, so they no longer describe it. Other domains' writes stay.
    pub fn discard(&self, domain: Domain) {
        self.changes
            .borrow_mut()
            .retain(|change| change.domain() != domain);
    }
}

impl Commit for PendingWrites {
    #[tracing::instrument(name = "Commit the Ledger writes", level = "debug", skip_all)]
    fn commit(&self) -> Committed {
        let mut wrote = self.written.take();
        let changes = std::mem::take(&mut *self.changes.borrow_mut());
        if changes.is_empty() {
            return Committed {
                wrote,
                cascaded: Vec::new(),
            };
        }
        let batch = Batch::new(changes);
        // A write queued by an adapter no recorder wraps is still the input's.
        for domain in batch.domains() {
            if !wrote.contains(&domain) {
                wrote.push(domain);
            }
        }
        let cascaded = batch.cascades();
        self.writer.write(batch);
        Committed { wrote, cascaded }
    }
}

/// The commit of in-memory ports, which are their own storage, so there is nowhere to send and no
/// file to cascade: it says only which domains were written.
#[derive(Debug, Default, Clone)]
pub struct InMemoryCommit {
    written: Written,
}

impl InMemoryCommit {
    /// The domains noted as written since the last commit, which the stores' recorders share.
    pub fn written(&self) -> &Written {
        &self.written
    }
}

impl Commit for InMemoryCommit {
    fn commit(&self) -> Committed {
        Committed {
            wrote: self.written.take(),
            cascaded: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use chrono::Utc;
    use lib_core::{UnitId, UnitKind};
    use lib_units::fake::{InMemoryPorts, InMemoryUnitUsage};
    use lib_units::interactor::AddUnit;
    use lib_units::{Precision, PriceLink};
    use sqlx::SqliteConnection;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::writer::{BoxFuture, WriteFailure};
    use crate::{Error, Result, UnitsAdapter};

    /// A second domain's write, test-only: a row in `notes`, whose `body` is `NOT NULL`, so a
    /// `None` body is a write SQLite refuses.
    struct AddNote(Option<&'static str>);

    impl Change for AddNote {
        fn domain(&self) -> Domain {
            Domain::Test
        }

        fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
            Box::pin(async move {
                // Unchecked: `notes` is made by the test, so no migration gives the macros it.
                sqlx::query("INSERT INTO notes (body) VALUES (?)")
                    .bind(self.0)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        }
    }

    /// The Units adapter and the notes "adapter" on one test database and one pending batch.
    struct Ledger {
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
        units: UnitsAdapter,
    }

    impl Ledger {
        fn open(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            writer
                .run(|pool| async move {
                    sqlx::query("CREATE TABLE notes (body TEXT NOT NULL)")
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the notes table is made");
            UnitsAdapter::seed_from(&writer, InMemoryPorts::sample(Utc::now()))
                .expect("the seed is written");
            let pending = PendingWrites::new(writer);
            let units = UnitsAdapter::load(pending.clone()).expect("the seed loads");
            Self {
                pending,
                failures,
                units,
            }
        }

        /// One Interactor call's worth of writes to both domains: a Unit, then a note.
        fn add_unit_and_note(&mut self, note: Option<&'static str>) {
            let usage = InMemoryUnitUsage::default();
            let ports = self.units.split_mut(&usage);
            AddUnit {
                id: UnitId::new(),
                code: "nzd".into(),
                name: "New Zealand dollar".into(),
                kind: UnitKind::Currency,
                precision: Precision::of_kind(UnitKind::Currency),
                link: PriceLink::manual(UnitId::SEED_AUD),
            }
            .run(ports.units, ports.price_sources)
            .expect("Manual entry exists");
            self.pending.push(AddNote(note));
        }

        fn stored_codes(&self) -> Vec<String> {
            let fresh = UnitsAdapter::load(PendingWrites::new(self.pending.writer().clone()))
                .expect("the rows load");
            fresh
                .units()
                .units()
                .into_iter()
                .map(|unit| unit.code)
                .collect()
        }

        fn stored_notes(&self) -> Vec<String> {
            self.pending
                .writer()
                .run(|pool| async move {
                    Ok(sqlx::query_scalar("SELECT body FROM notes")
                        .fetch_all(&pool)
                        .await?)
                })
                .expect("the notes load")
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn one_commit_lands_both_domains_writes(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut ledger = Ledger::open(options);

        ledger.add_unit_and_note(Some("paid"));
        let committed = ledger.pending.commit();
        ledger
            .pending
            .writer()
            .flush()
            .expect("the writer is running");

        assert!(ledger.stored_codes().contains(&"nzd".to_string()));
        assert_eq!(ledger.stored_notes(), ["paid"]);
        assert_eq!(committed.wrote, [Domain::Units, Domain::Test]);
        assert!(
            committed.cascaded.is_empty(),
            "nothing deleted, so nothing to reload"
        );
        assert!(ledger.failures.try_recv().is_err());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refused_write_lands_neither_domain_and_names_both(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut ledger = Ledger::open(options);

        ledger.add_unit_and_note(None);
        ledger.pending.commit();

        let failure = ledger.failures.recv().expect("the failure is reported");
        assert_eq!(failure.domains, [Domain::Units, Domain::Test]);
        assert!(matches!(failure.error, Error::Sqlx(_)));
        assert!(
            !ledger.stored_codes().contains(&"nzd".to_string()),
            "the Unit written before the refused note is rolled back with it"
        );
        assert!(ledger.stored_notes().is_empty());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_reload_discards_only_its_own_domains_writes(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut ledger = Ledger::open(options);

        ledger.add_unit_and_note(Some("kept"));
        ledger.units.reload().expect("the rows load");
        ledger.pending.commit();
        ledger
            .pending
            .writer()
            .flush()
            .expect("the writer is running");

        assert!(!ledger.stored_codes().contains(&"nzd".to_string()));
        assert_eq!(ledger.stored_notes(), ["kept"]);
    }
}
