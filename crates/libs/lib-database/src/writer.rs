//! The one writer thread every domain adapter saves through (ADR-0046). It owns a current-thread
//! `tokio` runtime and a one-connection pool, so the Clients' synchronous ports never wait on
//! `sqlx` and writes from every domain land in the order they were sent.
//!
//! Every adapter's changes from one Interactor call are sent as one [`Batch`]
//! ([`PendingWrites`](crate::commit::PendingWrites) gathers them), run in one transaction with
//! foreign keys on. When a batch fails, the writer drops every write queued behind it, since each
//! was made on top of memory that no longer matches the file, and reports a [`WriteFailure`] on
//! the channel [`Writer::spawn`] returns. The bin listens there and has each named domain's
//! adapter reload.
//!
//! [`Writer::run`] also queues blocking work, such as an adapter's load, behind the writes already
//! sent, so a reload sees every write before it.
//!
//! A Client starts its writer with [`Writer::open_ledger`], which also brings the file's schema up
//! to date, and [`Writer::flush`]es it before exiting so the last writes aren't lost with the
//! thread.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::mpsc;
use std::thread;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::runtime::Runtime;

use crate::{Error, Result};

/// The Client Ledger's schema, which [`Writer::open_ledger`] applies.
static CLIENT_MIGRATIONS: Migrator = sqlx::migrate!("./migrations/client");

/// A boxed future a [`Batch`] or a [`Writer::run`] job returns.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The domain a change belongs to, so a failure can name the adapters that must reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Units,
    Institutions,
    Accounts,
    Payees,
    Transactions,
    Bills,
    /// The test-only second adapter, which proves one commit spans domains.
    #[cfg(test)]
    Test,
}

/// One port write, saved inside its batch's transaction.
pub trait Change: Send + 'static {
    /// The domain whose adapter holds the row in memory, so must reload if the write is lost.
    fn domain(&self) -> Domain;

    /// Runs the write on `conn`, which is inside the batch's transaction.
    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>>;
}

/// One Interactor call's changes, from every adapter, applied in the order they were made in one
/// transaction: all of them land or none do.
pub struct Batch(Vec<Box<dyn Change>>);

impl Batch {
    pub fn new(changes: Vec<Box<dyn Change>>) -> Self {
        Self(changes)
    }

    /// Each domain the changes belong to, once, in the order first written.
    pub fn domains(&self) -> Vec<Domain> {
        let mut domains = Vec::new();
        self.add_domains_to(&mut domains);
        domains
    }

    /// Adds each of the batch's domains not already in `domains`, keeping their order.
    fn add_domains_to(&self, domains: &mut Vec<Domain>) {
        for change in &self.0 {
            if !domains.contains(&change.domain()) {
                domains.push(change.domain());
            }
        }
    }
}

/// A failed batch, with every domain whose writes were lost: the failed batch's and those of
/// the batches dropped behind it. Each of their adapters holds memory the file doesn't.
#[derive(Debug)]
pub struct WriteFailure {
    pub domains: Vec<Domain>,
    pub error: Error,
}

/// Blocking work queued with [`Writer::run`], which sends its own result back.
type RunJob = Box<dyn FnOnce(&Runtime, &SqlitePool) + Send>;

enum Job {
    /// A batch and the span it was sent from, so a failure logs under the call that made it.
    Write(Batch, tracing::Span),
    Run(RunJob),
}

/// A handle on the writer thread, cloned into each adapter. The thread stops once every handle is
/// dropped.
#[derive(Clone)]
pub struct Writer {
    jobs: mpsc::Sender<Job>,
}

impl Writer {
    /// Starts the writer on the database `options` names, blocking until it is connected.
    /// Migrations are the caller's: the writer only reads and writes rows.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the thread or its runtime can't start, [`Error::Sqlx`] if the
    /// database can't be opened, or [`Error::WriterStopped`] if the thread stops while opening.
    pub fn spawn(options: SqliteConnectOptions) -> Result<(Self, mpsc::Receiver<WriteFailure>)> {
        let (jobs, job_queue) = mpsc::channel();
        let (failures, failure_reports) = mpsc::channel();
        let (opened, open_result) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("ledger-writer".into())
            .spawn(move || {
                let started = open(options);
                let (runtime, pool) = match started {
                    Ok(started) => {
                        let _ = opened.send(Ok(()));
                        started
                    }
                    Err(error) => {
                        let _ = opened.send(Err(error));
                        return;
                    }
                };
                serve(&runtime, &pool, &job_queue, &failures);
                runtime.block_on(pool.close());
            })?;
        open_result.recv().map_err(|_| Error::WriterStopped)??;
        Ok((Self { jobs }, failure_reports))
    }

    /// Starts the writer on the Client Ledger file at `path`, creating the file and its directory
    /// if missing, and runs the Client migrations on it, blocking until both are done.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the directory can't be made, the errors of [`Self::spawn`] if the
    /// file can't be opened, or [`Error::Migration`] if the schema can't be brought up to date.
    #[tracing::instrument(name = "Open the Ledger", level = "debug", err)]
    pub fn open_ledger(path: &Path) -> Result<(Self, mpsc::Receiver<WriteFailure>)> {
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let (writer, failures) = Self::spawn(options)?;
        writer.run(|pool| async move { Ok(CLIENT_MIGRATIONS.run(&pool).await?) })?;
        Ok((writer, failures))
    }

    /// Blocks until every write already sent has landed or failed. The thread dies with the
    /// process, so a Client calls this before exiting.
    ///
    /// # Errors
    /// Returns [`Error::WriterStopped`] if the thread has stopped.
    pub fn flush(&self) -> Result<()> {
        self.run(|_| async { Ok(()) })
    }

    /// Queues `batch` behind every write already sent. Never waits: a failure is reported on the
    /// failure channel.
    pub fn write(&self, batch: Batch) {
        let domains = batch.domains();
        if self
            .jobs
            .send(Job::Write(batch, tracing::Span::current()))
            .is_err()
        {
            tracing::error!(?domains, "The writer thread has stopped; a write was lost");
        }
    }

    /// Runs `job` on the writer thread after every write already sent, blocking until it returns.
    /// Never call it from inside a batch or another job, which would wait on itself.
    ///
    /// # Errors
    /// Returns `job`'s error, or [`Error::WriterStopped`] if the thread has stopped.
    pub fn run<T, F, Fut>(&self, job: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(SqlitePool) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>>,
    {
        let (reply, result) = mpsc::sync_channel(1);
        let job = Job::Run(Box::new(move |runtime, pool| {
            let _ = reply.send(runtime.block_on(job(pool.clone())));
        }));
        self.jobs.send(job).map_err(|_| Error::WriterStopped)?;
        result.recv().map_err(|_| Error::WriterStopped)?
    }
}

fn open(options: SqliteConnectOptions) -> Result<(Runtime, SqlitePool)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let pool = runtime.block_on(
        SqlitePoolOptions::new()
            // One connection, so one writer, and no idle timer to outlive the runtime.
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options.foreign_keys(true)),
    )?;
    Ok((runtime, pool))
}

fn serve(
    runtime: &Runtime,
    pool: &SqlitePool,
    jobs: &mpsc::Receiver<Job>,
    failures: &mpsc::Sender<WriteFailure>,
) {
    while let Ok(job) = jobs.recv() {
        let (batch, sender) = match job {
            Job::Run(job) => {
                job(runtime, pool);
                continue;
            }
            Job::Write(batch, sender) => (batch, sender),
        };
        let mut domains = batch.domains();
        let span = tracing::debug_span!(parent: &sender, "Apply a write", ?domains);
        let _entered = span.enter();
        let Err(error) = runtime.block_on(apply(pool, batch)) else {
            continue;
        };
        tracing::error!(?domains, %error, "A write failed; dropping the writes queued behind it");
        let mut waiting = Vec::new();
        while let Ok(job) = jobs.try_recv() {
            match job {
                Job::Run(job) => waiting.push(job),
                Job::Write(dropped, _) => dropped.add_domains_to(&mut domains),
            }
        }
        // Reported before the waiting jobs run, so whoever they answer can already see it.
        let _ = failures.send(WriteFailure { domains, error });
        // Waiting jobs still run: an adapter's reload may already be among them.
        for job in waiting {
            job(runtime, pool);
        }
    }
}

async fn apply(pool: &SqlitePool, batch: Batch) -> Result<()> {
    let mut transaction = pool.begin().await?;
    for change in batch.0 {
        change.apply(&mut transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

/// A change that holds the writer until the test lets it go, so later writes queue behind it.
#[cfg(test)]
pub(crate) struct Gate(pub(crate) mpsc::Receiver<()>);

#[cfg(test)]
impl Gate {
    /// A batch of the gate alone, and the sender that lets it go.
    pub(crate) fn batch() -> (Batch, mpsc::Sender<()>) {
        let (release, gate) = mpsc::channel();
        (Batch::new(vec![Box::new(Self(gate))]), release)
    }
}

#[cfg(test)]
impl Change for Gate {
    fn domain(&self) -> Domain {
        Domain::Units
    }

    fn apply<'c>(self: Box<Self>, _conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            let _ = self.0.recv();
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A change that runs one SQL statement and records that it ran.
    struct Statement {
        sql: &'static str,
        ran: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Statement {
        fn batch(sql: &'static str, ran: &Arc<Mutex<Vec<&'static str>>>) -> Batch {
            Batch::new(vec![Box::new(Self {
                sql,
                ran: ran.clone(),
            })])
        }
    }

    impl Change for Statement {
        fn domain(&self) -> Domain {
            Domain::Units
        }

        fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
            Box::pin(async move {
                sqlx::query(self.sql).execute(&mut *conn).await?;
                self.ran.lock().expect("not poisoned").push(self.sql);
                Ok(())
            })
        }
    }

    #[sqlx::test(migrations = false)]
    async fn writes_land_in_order_and_a_run_sees_them(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, failures) = Writer::spawn(options).unwrap();
        let ran = Arc::new(Mutex::new(Vec::new()));
        for sql in [
            "CREATE TABLE notes (body TEXT NOT NULL)",
            "INSERT INTO notes VALUES ('first')",
            "INSERT INTO notes VALUES ('second')",
        ] {
            writer.write(Statement::batch(sql, &ran));
        }

        let bodies: Vec<String> = writer
            .run(|pool| async move {
                Ok(sqlx::query_scalar("SELECT body FROM notes ORDER BY rowid")
                    .fetch_all(&pool)
                    .await?)
            })
            .unwrap();

        assert_eq!(bodies, ["first", "second"]);
        assert_eq!(ran.lock().unwrap().len(), 3);
        assert!(failures.try_recv().is_err());
    }

    #[sqlx::test(migrations = false)]
    async fn a_failed_write_drops_the_writes_queued_behind_it_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, failures) = Writer::spawn(options).unwrap();
        let ran = Arc::new(Mutex::new(Vec::new()));
        let write = |sql| writer.write(Statement::batch(sql, &ran));
        write("CREATE TABLE notes (body TEXT NOT NULL)");
        let (gate, release) = Gate::batch();
        writer.write(gate);
        write("INSERT INTO notes VALUES (NULL)");
        write("INSERT INTO notes VALUES ('queued')");
        release.send(()).unwrap();

        let failure = failures.recv().unwrap();
        write("INSERT INTO notes VALUES ('after')");
        let notes: i64 = writer
            .run(|pool| async move {
                Ok(sqlx::query_scalar("SELECT COUNT(*) FROM notes")
                    .fetch_one(&pool)
                    .await?)
            })
            .unwrap();

        assert_eq!(failure.domains, [Domain::Units]);
        assert!(matches!(failure.error, Error::Sqlx(_)));
        assert_eq!(notes, 1, "only the write after the failure");
        assert!(failures.try_recv().is_err());
    }

    #[sqlx::test(migrations = false)]
    async fn flush_returns_once_the_writes_already_sent_have_landed(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).unwrap();
        let ran = Arc::new(Mutex::new(Vec::new()));
        let (gate, release) = Gate::batch();
        writer.write(gate);
        writer.write(Statement::batch(
            "CREATE TABLE notes (body TEXT NOT NULL)",
            &ran,
        ));
        release.send(()).unwrap();

        writer.flush().unwrap();

        assert_eq!(ran.lock().unwrap().len(), 1);
    }

    #[test]
    fn opening_a_new_ledger_creates_the_file_and_migrates_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("ledger.pldb");

        let (writer, _failures) = Writer::open_ledger(&path).unwrap();
        let units: i64 = writer
            .run(|pool| async move {
                Ok(sqlx::query_scalar("SELECT COUNT(*) FROM units")
                    .fetch_one(&pool)
                    .await?)
            })
            .unwrap();

        assert!(path.is_file());
        assert_eq!(units, 0);
    }

    #[test]
    fn opening_a_ledger_whose_directory_is_a_file_is_an_error() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().join("ledger.pldb");

        assert!(matches!(
            Writer::open_ledger(&path).map(|_| ()),
            Err(Error::Io(_))
        ));
    }

    #[test]
    fn spawning_on_a_database_that_cannot_open_is_an_error() {
        let options = SqliteConnectOptions::new()
            .filename("/nonexistent-directory/ledger.db")
            .create_if_missing(false);
        assert!(matches!(
            Writer::spawn(options).map(|_| ()),
            Err(Error::Sqlx(_))
        ));
    }
}
