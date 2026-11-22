//! The Ledger adapter: the Ledger's own Preferences, its name, Owner and Financial year start, on
//! the one-row `ledger` table (ADR-0061). The row's `base_unit_id` is the Units adapter's, so each
//! adapter reads and writes only its own columns of the one row.
//!
//! It loads the row into memory, blocking, as every adapter does (ADR-0046). Each edit updates
//! only its own column; creating a Ledger inserts the whole row ([`create_in`]), and the Sample
//! Ledger's seed does the same where there is none ([`seed_in`]).

use lib_core::{FinancialYearStart, LedgerName, Owner, UnitId};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain};
use crate::{Error, Result};

/// The Ledger name, Owner and Financial year start, as the `ledger` row holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRow {
    pub name: LedgerName,
    pub owner: Option<Owner>,
    pub financial_year_start: FinancialYearStart,
}

/// The Ledger's Preferences, held in memory.
pub struct LedgerAdapter {
    row: LedgerRow,
    pending: PendingWrites,
}

impl LedgerAdapter {
    /// Loads the `ledger` row through `pending`'s writer, blocking until it is read.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] if the file has no `ledger` row, as only a cut-short or
    /// hand-made file doesn't, [`Error::Validation`] for a value no Ledger is made with, or the
    /// error of the read.
    #[tracing::instrument(name = "Load the Ledger's Preferences", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let row = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self { row, pending })
    }

    /// Loads the row again, blocking. For after a failed write or a reload the Client asks for,
    /// so memory matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(
        name = "Reload the Ledger's Preferences",
        level = "debug",
        skip_all,
        err
    )]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.row = fresh.row;
        self.pending.discard(Domain::Ledger);
        Ok(())
    }

    pub fn row(&self) -> &LedgerRow {
        &self.row
    }

    pub fn set_name(&mut self, name: LedgerName) {
        self.row.name = name.clone();
        self.pending.push(LedgerChange::Name(name));
    }

    pub fn set_owner(&mut self, owner: Option<Owner>) {
        self.row.owner = owner.clone();
        self.pending.push(LedgerChange::Owner(owner));
    }

    pub fn set_financial_year_start(&mut self, start: FinancialYearStart) {
        self.row.financial_year_start = start;
        self.pending.push(LedgerChange::FinancialYearStart(start));
    }
}

/// A queued edit of one column of the `ledger` row.
enum LedgerChange {
    Name(LedgerName),
    Owner(Option<Owner>),
    FinancialYearStart(FinancialYearStart),
}

impl Change for LedgerChange {
    fn domain(&self) -> Domain {
        Domain::Ledger
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Name(name) => {
                    let name = name.as_str();
                    sqlx::query!("UPDATE ledger SET name = ? WHERE id = 1", name)
                        .execute(conn)
                        .await?;
                }
                Self::Owner(owner) => {
                    let owner = owner.as_ref().map(Owner::as_str);
                    sqlx::query!("UPDATE ledger SET owner = ? WHERE id = 1", owner)
                        .execute(conn)
                        .await?;
                }
                Self::FinancialYearStart(start) => {
                    let month = start.month();
                    sqlx::query!(
                        "UPDATE ledger SET financial_year_start = ? WHERE id = 1",
                        month
                    )
                    .execute(conn)
                    .await?;
                }
            }
            Ok(())
        })
    }
}

/// Inserts the whole `ledger` row on `conn`, for creating a Ledger: the one write of all four
/// columns, so the file never holds a row with only some of them.
///
/// # Errors
/// Returns the error of the write, such as a second row.
pub(crate) async fn create_in(
    conn: &mut SqliteConnection,
    row: &LedgerRow,
    base_unit: UnitId,
) -> Result<()> {
    let LedgerColumns { name, owner, month } = LedgerColumns::of(row);
    sqlx::query!(
        "INSERT INTO ledger (id, base_unit_id, name, owner, financial_year_start)
         VALUES (1, ?, ?, ?, ?)",
        base_unit,
        name,
        owner,
        month,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Inserts the whole `ledger` row on `conn` only when the file has none, so a re-seed keeps the
/// person's base Unit and Preferences.
///
/// # Errors
/// Returns the error of the write.
pub(crate) async fn seed_in(
    conn: &mut SqliteConnection,
    row: &LedgerRow,
    base_unit: UnitId,
) -> Result<()> {
    let LedgerColumns { name, owner, month } = LedgerColumns::of(row);
    sqlx::query!(
        "INSERT INTO ledger (id, base_unit_id, name, owner, financial_year_start)
         VALUES (1, ?, ?, ?, ?)
         ON CONFLICT (id) DO NOTHING",
        base_unit,
        name,
        owner,
        month,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// A [`LedgerRow`] as its columns bind.
struct LedgerColumns<'a> {
    name: &'a str,
    owner: Option<&'a str>,
    month: u32,
}

impl<'a> LedgerColumns<'a> {
    fn of(row: &'a LedgerRow) -> Self {
        Self {
            name: row.name.as_str(),
            owner: row.owner.as_ref().map(Owner::as_str),
            month: row.financial_year_start.month(),
        }
    }
}

async fn read(conn: &mut SqliteConnection) -> Result<LedgerRow> {
    let row = sqlx::query!(
        r#"SELECT
               name,
               owner,
               financial_year_start AS "financial_year_start!: u32"
           FROM ledger
           WHERE id = 1"#
    )
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| Error::NotFound("the Ledger has no ledger row".into()))?;
    // The table's CHECKs hold both, so these refuse only a file edited by hand.
    let name = LedgerName::new(&row.name)
        .map_err(|error| Error::Validation(format!("the Ledger name is invalid: {error}")))?;
    let financial_year_start =
        FinancialYearStart::new(row.financial_year_start).map_err(|error| {
            Error::Validation(format!("the Financial year start is invalid: {error}"))
        })?;
    Ok(LedgerRow {
        name,
        owner: row.owner.as_deref().and_then(Owner::new),
        financial_year_start,
    })
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::Writer;

    fn row(owner: Option<&str>, month: u32) -> LedgerRow {
        LedgerRow {
            name: LedgerName::new("Household").expect("a short, non-blank name"),
            owner: owner.and_then(Owner::new),
            financial_year_start: FinancialYearStart::new(month).expect("a month"),
        }
    }

    /// A test database with one Unit to be the base, the writer and the pending batch on it.
    fn database(options: SqliteConnectOptions) -> (Writer, PendingWrites) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        writer
            .run(|pool| async move {
                let units = lib_units::fake::InMemoryPorts::sample(chrono::Utc::now());
                let mut transaction = pool.begin().await?;
                crate::UnitsAdapter::seed_in(
                    &mut transaction,
                    &units.units.units,
                    &units.price_sources.records,
                )
                .await?;
                transaction.commit().await?;
                Ok(())
            })
            .expect("the Units are written");
        (writer.clone(), PendingWrites::new(writer))
    }

    fn create(writer: &Writer, row: LedgerRow) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.acquire().await?;
            create_in(&mut conn, &row, UnitId::SEED_AUD).await
        })
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_created_row_loads_with_its_name_owner_and_financial_year_start(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, pending) = database(options);
        create(&writer, row(Some("Ian Teda"), 7)).expect("the row is written");

        let adapter = LedgerAdapter::load(pending).expect("the row loads");

        assert_eq!(adapter.row(), &row(Some("Ian Teda"), 7));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn no_owner_is_stored_as_null(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        let (writer, pending) = database(options);
        create(&writer, row(None, 1)).expect("the row is written");

        let owner: Option<String> = writer
            .run(|pool| async move {
                Ok(sqlx::query_scalar("SELECT owner FROM ledger")
                    .fetch_one(&pool)
                    .await?)
            })
            .expect("the owner reads");

        assert_eq!(owner, None);
        assert_eq!(
            LedgerAdapter::load(pending)
                .expect("the row loads")
                .row()
                .owner,
            None
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_seed_keeps_the_row_already_there(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, pending) = database(options);
        create(&writer, row(Some("Ian Teda"), 7)).expect("the row is written");

        writer
            .run(|pool| async move {
                let mut conn = pool.acquire().await?;
                seed_in(&mut conn, &row(None, 1), UnitId::SEED_AUD).await
            })
            .expect("the seed is written");

        assert_eq!(
            LedgerAdapter::load(pending).expect("the row loads").row(),
            &row(Some("Ian Teda"), 7)
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_second_create_is_refused(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        let (writer, _pending) = database(options);
        create(&writer, row(None, 1)).expect("the row is written");

        assert!(create(&writer, row(None, 7)).is_err());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_file_with_no_ledger_row_is_not_found(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (_writer, pending) = database(options);

        assert!(matches!(
            LedgerAdapter::load(pending).map(|_| ()),
            Err(Error::NotFound(_))
        ));
    }

    /// The table refuses what the value types do, so a hand edit can't store a blank name or a
    /// thirteenth month either.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_table_refuses_a_blank_name_and_a_month_out_of_range(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _pending) = database(options);
        for (name, month) in [("  ", 1), ("Household", 0), ("Household", 13)] {
            let refused = writer.run(move |pool| async move {
                sqlx::query(
                    "INSERT INTO ledger (id, base_unit_id, name, owner, financial_year_start)
                     VALUES (1, ?, ?, NULL, ?)",
                )
                .bind(UnitId::SEED_AUD)
                .bind(name)
                .bind(month)
                .execute(&pool)
                .await?;
                Ok(())
            });
            assert!(refused.is_err(), "{name:?} {month}");
        }
    }
}
