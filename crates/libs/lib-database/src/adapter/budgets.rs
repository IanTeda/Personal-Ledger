//! The Budgets adapter: `lib-budgets`' `BudgetRepository` over the `budgets`, `budget_accounts` and
//! `budget_limit_records` tables (ADR-0046, #952).
//!
//! It loads every Budget with its Accounts and Category Limit chains into memory, blocking, and
//! serves them from there. A write changes memory at once and joins the [`PendingWrites`] every
//! adapter shares, which the bin commits after each input. Writing a Budget writes it whole: its
//! row, then its Accounts and records replaced with the ones it holds, so a chain it dropped is
//! gone. A Budget is archived, never deleted, so there is no delete.
//!
//! Deleting an Account cascades to its `budget_accounts` links, but a Budget in memory keeps the
//! id until it next loads, which `lib-budgets` ignores (`Budget::counts`). Writing such a Budget
//! again skips the id the file no longer has, rather than failing on its foreign key, so the file
//! stays as the cascade left it.

use chrono::NaiveDate;
use lib_budgets::{Budget, BudgetRepository, Limit, LimitRecord, Method, Rollover};
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period, UnitId};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum BudgetsChange {
    Upsert(Box<Budget>),
}

/// The Budgets domain, held in memory and saved through the shared writer.
pub struct BudgetsAdapter {
    rows: Vec<Budget>,
    pending: PendingWrites,
}

impl BudgetsAdapter {
    /// Loads every Budget with its Accounts and chains through `pending`'s writer, blocking until
    /// they are read. The adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read, or of a row naming a Budget that isn't stored.
    #[tracing::instrument(name = "Load the Budgets", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self { rows, pending })
    }

    /// Writes `budgets` whole in one transaction, blocking until it lands. Each is an upsert, so
    /// seeding again leaves the same rows. For after the Units, Accounts and Categories they name.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Budgets", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, budgets: Vec<Budget>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &budgets).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(conn: &mut SqliteConnection, budgets: &[Budget]) -> Result<()> {
        for budget in budgets {
            upsert(conn, budget).await?;
        }
        Ok(())
    }

    /// Whether the file holds no Budget, blocking: what [`crate::open_storage`] seeds.
    ///
    /// # Errors
    /// Returns the error of the read.
    pub(crate) fn is_empty(writer: &Writer) -> Result<bool> {
        writer.run(|pool| async move {
            let count = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM budgets"#)
                .fetch_one(&pool)
                .await?;
            Ok(count == 0)
        })
    }

    /// Drops the Budgets changes not yet committed and loads the rows again, blocking. For after a
    /// [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Budgets`], so memory matches
    /// the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Budgets", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.rows = fresh.rows;
        self.pending.discard(Domain::Budgets);
        Ok(())
    }
}

impl BudgetRepository for BudgetsAdapter {
    fn budgets(&self) -> Vec<Budget> {
        self.rows.clone()
    }

    fn upsert_budget(&mut self, budget: &Budget) {
        match self.rows.iter_mut().find(|row| row.id == budget.id) {
            Some(row) => *row = budget.clone(),
            None => self.rows.push(budget.clone()),
        }
        self.pending
            .push(BudgetsChange::Upsert(Box::new(budget.clone())));
    }
}

impl Change for BudgetsChange {
    fn domain(&self) -> Domain {
        Domain::Budgets
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(budget) => upsert(conn, &budget).await,
            }
        })
    }
}

fn method_text(method: Method) -> &'static str {
    match method {
        Method::Limits => "limits",
    }
}

fn method_of(text: &str) -> Result<Method> {
    match text {
        "limits" => Ok(Method::Limits),
        other => Err(Error::Validation(format!(
            "unknown Budget method {other:?}"
        ))),
    }
}

fn rollover_text(rollover: Rollover) -> &'static str {
    match rollover {
        Rollover::None => "none",
        Rollover::CarryUnspent => "carry_unspent",
        Rollover::CarryBoth => "carry_both",
    }
}

fn rollover_of(text: &str) -> Result<Rollover> {
    match text {
        "none" => Ok(Rollover::None),
        "carry_unspent" => Ok(Rollover::CarryUnspent),
        "carry_both" => Ok(Rollover::CarryBoth),
        other => Err(Error::Validation(format!("unknown Rollover {other:?}"))),
    }
}

/// A record's kind, amount and Rollover as their columns hold them.
fn limit_columns(limit: &Limit) -> (&'static str, Option<&Money>, &'static str) {
    match limit {
        Limit::Onward { amount, rollover } => ("onward", Some(amount), rollover_text(*rollover)),
        Limit::MonthOnly { amount, rollover } => {
            ("month_only", Some(amount), rollover_text(*rollover))
        }
        Limit::Stop => ("stop", None, rollover_text(Rollover::None)),
    }
}

fn limit_of(kind: &str, amount: Option<Money>, rollover: &str) -> Result<Limit> {
    let rollover = rollover_of(rollover)?;
    match (kind, amount) {
        ("onward", Some(amount)) => Ok(Limit::Onward { amount, rollover }),
        ("month_only", Some(amount)) => Ok(Limit::MonthOnly { amount, rollover }),
        ("stop", None) => Ok(Limit::Stop),
        (kind, amount) => Err(Error::Validation(format!(
            "a {kind:?} record with amount {amount:?}"
        ))),
    }
}

/// Writes the Budget's row, then replaces its Accounts and records with the ones it holds, in
/// order. An Account id the file no longer has is skipped (see the module docs).
async fn upsert(conn: &mut SqliteConnection, budget: &Budget) -> Result<()> {
    let method = method_text(budget.method);
    sqlx::query!(
        "INSERT INTO budgets (id, name, method, unit_id, is_default, archived_on)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             method = excluded.method,
             unit_id = excluded.unit_id,
             is_default = excluded.is_default,
             archived_on = excluded.archived_on",
        budget.id,
        budget.name,
        method,
        budget.unit,
        budget.is_default,
        budget.archived_at,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM budget_accounts WHERE budget_id = ?", budget.id)
        .execute(&mut *conn)
        .await?;
    for account in &budget.account_ids {
        sqlx::query!(
            "INSERT INTO budget_accounts (budget_id, account_id)
             SELECT ?1, ?2 WHERE EXISTS (SELECT 1 FROM accounts WHERE id = ?2)",
            budget.id,
            account,
        )
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query!(
        "DELETE FROM budget_limit_records WHERE budget_id = ?",
        budget.id
    )
    .execute(&mut *conn)
    .await?;
    for (category, chain) in &budget.limits {
        for record in chain {
            let month = record.month.first_day();
            let (kind, amount, rollover) = limit_columns(&record.limit);
            sqlx::query!(
                "INSERT INTO budget_limit_records
                     (budget_id, category_id, month, kind, amount, rollover)
                 VALUES (?, ?, ?, ?, ?, ?)",
                budget.id,
                category,
                month,
                kind,
                amount,
                rollover,
            )
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Budget>> {
    let mut budgets = Vec::new();
    for row in sqlx::query!(
        r#"SELECT
               id          AS "id!: BudgetId",
               name,
               method,
               unit_id     AS "unit_id!: UnitId",
               is_default,
               archived_on AS "archived_on?: NaiveDate"
           FROM budgets
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        budgets.push(Budget {
            id: row.id,
            name: row.name,
            method: method_of(&row.method)?,
            unit: row.unit_id,
            account_ids: Vec::new(),
            is_default: row.is_default,
            archived_at: row.archived_on,
            limits: Default::default(),
        });
    }
    let stored = |budgets: &mut Vec<Budget>, id: BudgetId| -> Result<usize> {
        budgets
            .iter()
            .position(|budget| budget.id == id)
            .ok_or_else(|| {
                Error::Validation(format!("a row names Budget {id}, which isn't stored"))
            })
    };
    for link in sqlx::query!(
        r#"SELECT budget_id AS "budget_id!: BudgetId", account_id AS "account_id!: AccountId"
           FROM budget_accounts
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        let at = stored(&mut budgets, link.budget_id)?;
        budgets[at].account_ids.push(link.account_id);
    }
    for record in sqlx::query!(
        r#"SELECT
               budget_id   AS "budget_id!: BudgetId",
               category_id AS "category_id!: CategoryId",
               month       AS "month!: NaiveDate",
               kind,
               amount      AS "amount?: Money",
               rollover
           FROM budget_limit_records
           ORDER BY month"#
    )
    .fetch_all(&mut *conn)
    .await?
    {
        let at = stored(&mut budgets, record.budget_id)?;
        budgets[at]
            .limits
            .entry(record.category_id)
            .or_default()
            .push(LimitRecord {
                month: Period::of(record.month),
                limit: limit_of(&record.kind, record.amount, &record.rollover)?,
            });
    }
    Ok(budgets)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_budgets::contract::{self, PortsFactory, stub_budgets};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;

    /// The adapter on one test database, with its pending batch. The stub's Units, Accounts and
    /// Categories are stored first, which a Budget names. A reload commits, loads afresh, and fails
    /// the check if the writer reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_stub_references(&writer);
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

        /// Runs `sql` binding `id`, returning whether the file took it.
        fn delete<Id>(&self, sql: &'static str, id: Id) -> bool
        where
            Id: for<'q> sqlx::Encode<'q, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + 'static,
        {
            self.writer
                .run(move |pool| async move {
                    sqlx::query(sql).bind(id).execute(&pool).await?;
                    Ok(())
                })
                .is_ok()
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = BudgetsAdapter;

        fn create(&mut self, seed: Vec<Budget>) -> BudgetsAdapter {
            BudgetsAdapter::seed(&self.writer, seed).expect("the seed is written");
            BudgetsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: BudgetsAdapter) -> BudgetsAdapter {
            self.pending.commit();
            let fresh = BudgetsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut BudgetsAdapter) {
            self.pending.commit();
        }

        fn delete_account(&mut self, _ports: &mut BudgetsAdapter, id: AccountId) {
            assert!(
                self.delete("DELETE FROM accounts WHERE id = ?", id),
                "nothing else names the Account"
            );
        }

        fn delete_category(&mut self, _ports: &mut BudgetsAdapter, id: CategoryId) -> bool {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            self.delete("DELETE FROM categories WHERE id = ?", id)
        }

        fn break_writes(&mut self, _ports: &mut BudgetsAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_budgets BEFORE INSERT ON budgets
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: BudgetsAdapter) -> (BudgetsAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Budgets])
                .count();
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn written_rows_reload_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::written_rows_reload_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_chain_is_replaced_whole(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_chain_is_replaced_whole(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn one_budget_is_the_default(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::one_budget_is_the_default(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_account_leaves_every_budget(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_deleted_account_leaves_every_budget(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_category_a_chain_names_cannot_be_deleted(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_category_a_chain_names_cannot_be_deleted(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The partial unique index backs the rule: a write that would leave two defaults fails in the
    /// file, and the Budgets reload as they were.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_second_default(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut budgets = factory.create(stub_budgets());
        let mut joint = budgets.budget(BudgetId::seed(2)).expect("a stub Budget");
        joint.is_default = true;

        budgets.upsert_budget(&joint);
        factory.pending.commit();
        let failures = factory.failures();
        assert_eq!(failures.len(), 1, "the second default is refused");
        assert_eq!(failures[0].domains, [Domain::Budgets]);
        budgets.reload().expect("the rows load");
        assert_eq!(budgets.budgets(), stub_budgets());
    }

    /// A Budget still holding a deleted Account's id, as one in memory does until it reloads, writes
    /// without it rather than failing.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_budget_holding_a_deleted_accounts_id_still_writes(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut budgets = factory.create(stub_budgets());
        let gone = AccountId::seed(2);
        factory.delete_account(&mut budgets, gone);
        let mut joint = budgets.budget(BudgetId::seed(2)).expect("a stub Budget");
        assert!(joint.account_ids.contains(&gone), "memory keeps the id");

        joint.name = "Shared house".into();
        budgets.upsert_budget(&joint);
        let budgets = factory.reload(budgets);

        let joint = budgets.budget(BudgetId::seed(2)).expect("a stub Budget");
        assert_eq!(joint.name, "Shared house");
        assert!(!joint.account_ids.contains(&gone));
    }

    /// A Budget names its Unit by foreign key with no cascade, so a Unit a Budget uses can't be
    /// deleted: a Unit nothing else names, only while a Budget counts in it.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_deleting_a_unit_a_budget_names(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let unit = crate::seed::fake_units(1, chrono::Utc::now()).remove(0);
        let unit_id = unit.id;
        crate::UnitsAdapter::seed(&factory.writer, vec![unit], Vec::new(), UnitId::SEED_AUD)
            .expect("the Unit is written");
        let mut budgets = factory.create(stub_budgets());
        let mut joint = budgets.budget(BudgetId::seed(2)).expect("a stub Budget");
        joint.unit = unit_id;
        joint.account_ids.clear();
        budgets.upsert_budget(&joint);
        let mut budgets = factory.reload(budgets);

        assert!(!factory.delete("DELETE FROM units WHERE id = ?", unit_id));

        joint.unit = UnitId::SEED_AUD;
        budgets.upsert_budget(&joint);
        factory.reload(budgets);
        assert!(factory.delete("DELETE FROM units WHERE id = ?", unit_id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_budgets(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        assert!(BudgetsAdapter::is_empty(&writer).expect("it counts"));
        let adapter = BudgetsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.budgets().is_empty());
    }
}
