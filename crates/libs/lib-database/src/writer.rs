//! The one writer thread every domain adapter saves through (ADR-0046). It owns a current-thread
//! `tokio` runtime and a one-connection pool, so the Clients' synchronous ports never wait on
//! `sqlx` and writes from every domain land in the order they were sent.
//!
//! An adapter sends each Interactor call's changes as one [`Batch`], run in one transaction with
//! foreign keys on. When a batch fails, the writer drops every write queued behind it, since each
//! was made on top of memory that no longer matches the file, and reports a [`WriteFailure`] on
//! the channel [`Writer::spawn`] returns. The bin listens there and has each named domain's
//! adapter reload.
//!
//! [`Writer::run`] also queues blocking work, such as an adapter's load, behind the writes already
//! sent, so a reload sees every write before it.

use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::thread;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqliteConnection, SqlitePool};
use tokio::runtime::Runtime;

use crate::{Error, Result};

/// A boxed future a [`Batch`] or a [`Writer::run`] job returns.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The domain a batch belongs to, so a failure can name the adapters that must reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Units,
}

/// One Interactor call's changes to one domain, applied in one transaction.
pub trait Batch: Send + 'static {
    fn domain(&self) -> Domain;

    /// Runs the changes, in order, on `conn`, which is inside the batch's transaction.
    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>>;
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
    Write(Box<dyn Batch>),
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
    /// Returns [`Error::Connection`] if the runtime can't start or the database can't be opened.
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
            })
            .map_err(|e| Error::Connection(format!("Failed to start the writer thread: {e}")))?;
        open_result
            .recv()
            .map_err(|_| Error::Connection("The writer thread stopped while opening".into()))??;
        Ok((Self { jobs }, failure_reports))
    }

    /// Queues `batch` behind every write already sent. Never waits: a failure is reported on the
    /// failure channel.
    pub fn write(&self, batch: Box<dyn Batch>) {
        let domain = batch.domain();
        if self.jobs.send(Job::Write(batch)).is_err() {
            tracing::error!(?domain, "The writer thread has stopped; a write was lost");
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
        .build()
        .map_err(|e| Error::Connection(format!("Failed to start the writer runtime: {e}")))?;
    let pool = runtime
        .block_on(
            SqlitePoolOptions::new()
                // One connection, so one writer, and no idle timer to outlive the runtime.
                .max_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .connect_with(options.foreign_keys(true)),
        )
        .map_err(|e| Error::Connection(format!("Failed to open the database: {e}")))?;
    Ok((runtime, pool))
}

fn serve(
    runtime: &Runtime,
    pool: &SqlitePool,
    jobs: &mpsc::Receiver<Job>,
    failures: &mpsc::Sender<WriteFailure>,
) {
    while let Ok(job) = jobs.recv() {
        let batch = match job {
            Job::Run(job) => {
                job(runtime, pool);
                continue;
            }
            Job::Write(batch) => batch,
        };
        let domain = batch.domain();
        let Err(error) = runtime.block_on(apply(pool, batch)) else {
            continue;
        };
        tracing::error!(?domain, %error, "A write failed; dropping the writes queued behind it");
        let mut domains = vec![domain];
        let mut waiting = Vec::new();
        while let Ok(job) = jobs.try_recv() {
            match job {
                Job::Run(job) => waiting.push(job),
                Job::Write(dropped) => {
                    if !domains.contains(&dropped.domain()) {
                        domains.push(dropped.domain());
                    }
                }
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

async fn apply(pool: &SqlitePool, batch: Box<dyn Batch>) -> Result<()> {
    let mut transaction = pool.begin().await?;
    batch.apply(&mut transaction).await?;
    transaction.commit().await?;
    Ok(())
}

/// A batch that holds the writer until the test lets it go, so later writes queue behind it.
#[cfg(test)]
pub(crate) struct Gate(pub(crate) mpsc::Receiver<()>);

#[cfg(test)]
impl Batch for Gate {
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

    /// A batch that runs one SQL statement and records that it ran.
    struct Statement {
        sql: &'static str,
        ran: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Batch for Statement {
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
            writer.write(Box::new(Statement {
                sql,
                ran: ran.clone(),
            }));
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
        let write = |sql| {
            writer.write(Box::new(Statement {
                sql,
                ran: ran.clone(),
            }))
        };
        write("CREATE TABLE notes (body TEXT NOT NULL)");
        let (release, gate) = mpsc::channel();
        writer.write(Box::new(Gate(gate)));
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

    #[test]
    fn spawning_on_a_database_that_cannot_open_is_an_error() {
        let options = SqliteConnectOptions::new()
            .filename("/nonexistent-directory/ledger.db")
            .create_if_missing(false);
        assert!(matches!(
            Writer::spawn(options).map(|_| ()),
            Err(Error::Connection(_))
        ));
    }
}
