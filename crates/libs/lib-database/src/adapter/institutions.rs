//! The Institutions adapter: `lib-institutions`' repository port over the `institutions` table
//! (ADR-0046).
//!
//! It loads every Institution into memory, blocking, and serves the rows from there, in the order
//! they were added. A port write changes memory at once and joins the [`PendingWrites`] every
//! adapter shares, which the bin commits after each input. The add rule refuses a blank or taken
//! name before anything is written; the table's unique name backs it. The delete rule refuses an
//! Institution an Account names; the `accounts` foreign key (no cascade) backs it, so a delete that
//! skips the rule fails in the file, is reported against the Institutions domain and is reloaded.

use lib_core::InstitutionId;
use lib_institutions::{Institution, InstitutionRepository};
use sqlx::SqliteConnection;

use crate::Result;
use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};

/// One port write.
#[derive(Debug, Clone)]
struct UpsertInstitution(Institution);

/// One port delete.
#[derive(Debug, Clone)]
struct DeleteInstitutionRow(InstitutionId);

/// The Institutions domain, held in memory and saved through the shared writer.
pub struct InstitutionsAdapter {
    rows: Vec<Institution>,
    pending: PendingWrites,
}

impl InstitutionsAdapter {
    /// Loads every Institution through `pending`'s writer, blocking until they are read. The
    /// adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Institutions", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self { rows, pending })
    }

    /// Writes `institutions` in one transaction, blocking until it lands. Each is an upsert, so
    /// seeding again leaves the same rows. For before the Accounts that name them.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Institutions", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, institutions: Vec<Institution>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &institutions).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(
        conn: &mut SqliteConnection,
        institutions: &[Institution],
    ) -> Result<()> {
        for institution in institutions {
            upsert(conn, institution).await?;
        }
        Ok(())
    }

    /// Drops the Institutions changes not yet committed and loads the rows again, blocking. For
    /// after a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Institutions`], so
    /// memory matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Institutions", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.rows = fresh.rows;
        self.pending.discard(Domain::Institutions);
        Ok(())
    }
}

impl InstitutionRepository for InstitutionsAdapter {
    fn institutions(&self) -> Vec<Institution> {
        self.rows.clone()
    }

    fn upsert_institution(&mut self, institution: Institution) {
        match self.rows.iter_mut().find(|row| row.id == institution.id) {
            Some(row) => *row = institution.clone(),
            None => self.rows.push(institution.clone()),
        }
        self.pending.push(UpsertInstitution(institution));
    }

    fn delete_institution(&mut self, id: InstitutionId) {
        let before = self.rows.len();
        self.rows.retain(|row| row.id != id);
        if self.rows.len() != before {
            self.pending.push(DeleteInstitutionRow(id));
        }
    }
}

impl Change for UpsertInstitution {
    fn domain(&self) -> Domain {
        Domain::Institutions
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move { upsert(conn, &self.0).await })
    }
}

impl Change for DeleteInstitutionRow {
    fn domain(&self) -> Domain {
        Domain::Institutions
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            sqlx::query!("DELETE FROM institutions WHERE id = ?", self.0)
                .execute(&mut *conn)
                .await?;
            Ok(())
        })
    }
}

async fn upsert(conn: &mut SqliteConnection, institution: &Institution) -> Result<()> {
    sqlx::query!(
        "INSERT INTO institutions (id, name, products)
         VALUES (?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             products = excluded.products",
        institution.id,
        institution.name,
        institution.products,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Institution>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: InstitutionId", name, products
           FROM institutions
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Institution {
        id: row.id,
        name: row.name,
        products: row.products,
    })
    .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_institutions::contract::{self, PortsFactory};
    use lib_institutions::default_institutions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;

    /// The adapter on one test database, with its pending batch. A reload commits, loads afresh,
    /// and fails the check if the writer reported a failure, so a write SQLite refuses can't pass
    /// unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// Settles the writes sent so far and returns the failures reported since the last call.
        fn failures(&self) -> Vec<WriteFailure> {
            self.writer.flush().expect("the writer is running");
            self.failures.try_iter().collect()
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = InstitutionsAdapter;

        fn create(&mut self, seed: Vec<Institution>) -> InstitutionsAdapter {
            InstitutionsAdapter::seed(&self.writer, seed).expect("the seed is written");
            InstitutionsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: InstitutionsAdapter) -> InstitutionsAdapter {
            self.pending.commit();
            let fresh = InstitutionsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut InstitutionsAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut InstitutionsAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_institutions BEFORE INSERT ON institutions
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: InstitutionsAdapter) -> (InstitutionsAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Institutions])
                .count();
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_added_institution_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_added_institution_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn institutions_list_in_the_order_added(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::institutions_list_in_the_order_added(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_blank_or_taken_name_is_refused_and_writes_nothing(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_blank_or_taken_name_is_refused_and_writes_nothing(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_edited_institution_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_edited_institution_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_clashing_rename_is_refused_and_writes_nothing(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_clashing_rename_is_refused_and_writes_nothing(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_case_only_rename_is_accepted(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_case_only_rename_is_accepted(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_institution_is_gone_after_a_reload(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_deleted_institution_is_gone_after_a_reload(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn deleting_one_in_use_is_refused_and_writes_nothing(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::deleting_one_in_use_is_refused_and_writes_nothing(&mut AdapterFactory::new(
            options,
        ));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The add rule is the person-facing refusal; the unique name backs it, ignoring case, so a
    /// write that skips the rule fails in the file and the first Institution stays alone.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_name_another_institution_holds_in_any_case(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut institutions = factory.create(default_institutions());
        let stored = institutions.institutions();

        institutions.upsert_institution(Institution {
            id: InstitutionId::new(),
            name: "anz banking group".into(),
            products: "savings".into(),
        });
        factory.pending.commit();

        let failures = factory.failures();
        assert_eq!(failures.len(), 1, "the taken name is refused");
        assert_eq!(failures[0].domains, [Domain::Institutions]);
        institutions.reload().expect("the rows load");
        assert_eq!(institutions.institutions(), stored);
    }
}
