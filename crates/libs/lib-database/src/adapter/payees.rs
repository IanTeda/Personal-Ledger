//! The Payees adapter: `lib-payees`' Payee repository and `PayeeUsage` ports over the `payees` and
//! `payee_aliases` tables, and `lib-bills`' `BillPayees` read port from the same rows (ADR-0046).
//!
//! It loads every Payee and its aliases into memory, blocking, and serves the rows from there. A
//! port write changes memory at once and joins the [`PendingWrites`] every adapter shares, which
//! the bin commits after each input. Writing a Payee rewrites its aliases, in the order they are
//! held, so they read back in that order; deleting one takes its aliases with it (the schema's
//! cascade).
//!
//! Usage is other domains' rows, so [`PayeeUsage`] counts the `splits` and `bill_plans` naming a
//! Payee in the file, blocking: what the last commit left, not a write still pending. A Payee
//! those tables name can't be deleted (their foreign keys), which backstops the delete refusal. A
//! count that can't be read is unknown, never unused (#840).

use lib_bills::{BillPayee, BillPayees};
use lib_core::{CategoryId, PayeeId, RowID};
use lib_payees::{Payee, PayeeRepository, PayeeUsage, PayeeUse, PortsMut};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum PayeesChange {
    Upsert(Payee),
    Delete(PayeeId),
}

/// The Payees domain, held in memory and saved through the shared writer.
pub struct PayeesAdapter {
    payees: StoredPayees,
    usage: StoredUsage,
}

/// The rows, with the batch their writes join.
struct StoredPayees {
    rows: Vec<Payee>,
    pending: PendingWrites,
}

/// Counts in the file, read through the writer.
struct StoredUsage {
    writer: Writer,
}

impl PayeesAdapter {
    /// Loads every Payee with its aliases through `pending`'s writer, blocking until they are
    /// read. The adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Payees", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self {
            usage: StoredUsage {
                writer: pending.writer().clone(),
            },
            payees: StoredPayees { rows, pending },
        })
    }

    /// Writes `payees` with their aliases in one transaction, blocking until it lands. Each is an
    /// upsert, so seeding again leaves the same rows. For before the Splits and Bill Plans that
    /// name them.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Payees", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, payees: Vec<Payee>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            for payee in &payees {
                upsert(&mut conn, payee).await?;
            }
            conn.commit().await?;
            Ok(())
        })
    }

    /// Drops the Payees changes not yet committed and loads the rows again, blocking. For after a
    /// [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Payees`], so memory matches
    /// the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Payees", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.payees.pending.clone())?;
        self.payees.rows = fresh.payees.rows;
        self.payees.pending.discard(Domain::Payees);
        Ok(())
    }

    pub fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    pub fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        &mut self.payees
    }

    pub fn usage(&self) -> &dyn PayeeUsage {
        &self.usage
    }

    /// The repository and usage, for one write.
    pub fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            payees: &mut self.payees,
            usage: &self.usage,
        }
    }
}

impl PayeeRepository for StoredPayees {
    fn payees(&self) -> Vec<Payee> {
        self.rows.clone()
    }

    fn upsert_payee(&mut self, payee: Payee) {
        match self.rows.iter_mut().find(|row| row.id == payee.id) {
            Some(row) => *row = payee.clone(),
            None => self.rows.push(payee.clone()),
        }
        self.pending.push(PayeesChange::Upsert(payee));
    }

    fn delete_payee(&mut self, id: PayeeId) {
        self.rows.retain(|row| row.id != id);
        self.pending.push(PayeesChange::Delete(id));
    }
}

impl PayeeUsage for StoredUsage {
    /// A count that can't be read is logged and answered as unknown, so the Payee is never offered
    /// for deleting while it may be in use (#840).
    fn payee_usage(&self, id: PayeeId) -> Option<PayeeUse> {
        self.writer
            .run(move |pool| async move {
                let mut conn = pool.acquire().await?;
                count_usage(&mut conn, id).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, payee = %id, "Failed to count what names a Payee");
            })
            .ok()
    }
}

impl BillPayees for PayeesAdapter {
    fn bill_payees(&self) -> Vec<BillPayee> {
        bill_payees(&self.payees.rows)
    }
}

/// `payees` as Bills reads them.
pub fn bill_payees(payees: &[Payee]) -> Vec<BillPayee> {
    payees
        .iter()
        .map(|payee| BillPayee {
            id: payee.id,
            name: payee.name.clone(),
            is_active: payee.is_active,
        })
        .collect()
}

impl Change for PayeesChange {
    fn domain(&self) -> Domain {
        Domain::Payees
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(payee) => upsert(conn, &payee).await,
                Self::Delete(id) => delete(conn, id).await,
            }
        })
    }
}

/// Writes the Payee's row, then replaces its aliases with the ones it holds, in order.
async fn upsert(conn: &mut SqliteConnection, payee: &Payee) -> Result<()> {
    sqlx::query!(
        "INSERT INTO payees (id, name, default_category_id, is_active)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             default_category_id = excluded.default_category_id,
             is_active = excluded.is_active",
        payee.id,
        payee.name,
        payee.default_category,
        payee.is_active,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM payee_aliases WHERE payee_id = ?", payee.id)
        .execute(&mut *conn)
        .await?;
    for pattern in &payee.aliases {
        let id = RowID::new();
        sqlx::query!(
            "INSERT INTO payee_aliases (id, payee_id, pattern) VALUES (?, ?, ?)",
            id,
            payee.id,
            pattern,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn delete(conn: &mut SqliteConnection, id: PayeeId) -> Result<()> {
    sqlx::query!("DELETE FROM payees WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn count_usage(conn: &mut SqliteConnection, id: PayeeId) -> Result<PayeeUse> {
    let row = sqlx::query!(
        r#"SELECT
               (SELECT COUNT(*) FROM splits WHERE payee_id = ?1)     AS "splits!: i64",
               (SELECT COUNT(*) FROM bill_plans WHERE payee_id = ?1) AS "bill_plans!: i64""#,
        id,
    )
    .fetch_one(&mut *conn)
    .await?;
    let count = |n: i64| {
        usize::try_from(n).map_err(|_| Error::Validation(format!("Payee {id}'s usage is {n}")))
    };
    Ok(PayeeUse {
        splits: count(row.splits)?,
        bill_plans: count(row.bill_plans)?,
    })
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Payee>> {
    let mut payees: Vec<Payee> = sqlx::query!(
        r#"SELECT
               id                  AS "id!: PayeeId",
               name,
               default_category_id AS "default_category_id?: CategoryId",
               is_active
           FROM payees
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Payee {
        id: row.id,
        name: row.name,
        aliases: Vec::new(),
        default_category: row.default_category_id,
        is_active: row.is_active,
    })
    .collect();
    let aliases = sqlx::query!(
        r#"SELECT payee_id AS "payee_id!: PayeeId", pattern
           FROM payee_aliases
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?;
    for alias in aliases {
        let payee = payees
            .iter_mut()
            .find(|payee| payee.id == alias.payee_id)
            .ok_or_else(|| {
                Error::Validation(format!(
                    "alias {:?} names Payee {}, which isn't stored",
                    alias.pattern, alias.payee_id
                ))
            })?;
        payee.aliases.push(alias.pattern);
    }
    Ok(payees)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_bills::{AmountKind, BillPlan, Recurrence};
    use lib_core::{AccountId, BillPlanId, Money, TransactionId, TransactionStatus, UnitId};
    use lib_payees::contract::{self, PortsFactory, StoredPorts};
    use lib_payees::default_payees;
    use lib_transactions::{Split, Transaction};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{BillsAdapter, TransactionsAdapter};

    impl StoredPorts for PayeesAdapter {
        fn split_mut(&mut self) -> PortsMut<'_> {
            PayeesAdapter::split_mut(self)
        }
    }

    /// The adapter on one test database, with its pending batch. The stub's Units, Accounts and
    /// Categories are stored first, which a Bill Plan, a Split or a Payee's default names too. A reload commits, loads
    /// afresh, and fails the check if the writer reported a failure, so a write SQLite refuses
    /// can't pass unseen.
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
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// A Transaction with `splits` Splits, each naming `payee`.
    fn transaction_naming(payee: Option<PayeeId>, splits: usize) -> Transaction {
        Transaction {
            id: TransactionId::new(),
            date: day(),
            account_id: AccountId::seed(1),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: (0..splits)
                .map(|_| Split {
                    amount: Money(BigDecimal::from(-10)),
                    category_id: CategoryId::seed(7),
                    payee_id: payee,
                    tag_ids: Vec::new(),
                })
                .collect(),
        }
    }

    fn plan_naming(payee: Option<PayeeId>) -> BillPlan {
        BillPlan {
            id: BillPlanId::new(),
            name: "Plan".into(),
            category_id: CategoryId::seed(4),
            unit: UnitId::SEED_AUD,
            account_id: AccountId::seed(1),
            payee_id: payee,
            planned_amount: Money(BigDecimal::from(10)),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::Monthly,
            first_due: day(),
            ends_on: None,
            attention_lead: None,
            is_active: true,
            generate_from: day(),
        }
    }

    impl PortsFactory for AdapterFactory {
        type Ports = PayeesAdapter;

        fn create(&mut self, seed: Vec<Payee>) -> PayeesAdapter {
            PayeesAdapter::seed(&self.writer, seed).expect("the seed is written");
            PayeesAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: PayeesAdapter) -> PayeesAdapter {
            self.pending.commit();
            let fresh = PayeesAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut PayeesAdapter) {
            self.pending.commit();
        }

        fn name_payee(&mut self, _ports: &mut PayeesAdapter, id: PayeeId, uses: PayeeUse) {
            if uses.splits > 0 {
                TransactionsAdapter::seed(
                    &self.writer,
                    vec![transaction_naming(Some(id), uses.splits)],
                )
                .expect("the Splits are written");
            }
            let plans = (0..uses.bill_plans)
                .map(|_| plan_naming(Some(id)))
                .collect();
            BillsAdapter::seed(&self.writer, plans, Vec::new())
                .expect("the Bill Plans are written");
        }

        fn break_writes(&mut self, _ports: &mut PayeesAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_payees BEFORE INSERT ON payees
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: PayeesAdapter) -> (PayeesAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Payees])
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
    async fn an_alias_another_payee_owns_is_refused_and_writes_nothing(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_alias_another_payee_owns_is_refused_and_writes_nothing(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_takes_its_aliases(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_delete_takes_its_aliases(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn payee_usage_counts_and_refuses_the_delete(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::payee_usage_counts_and_refuses_the_delete(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The Interactor's refusal is the person-facing rule; the foreign keys back it, so a delete
    /// that skips it fails in the file and the Payee stays.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_deleting_a_payee_a_split_or_bill_plan_names(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut payees = factory.create(default_payees());
        let (by_split, by_plan) = (PayeeId::seed(1), PayeeId::seed(2));
        let mut ports = PayeesAdapter::load(factory.pending.clone()).expect("the rows load");
        factory.name_payee(
            &mut ports,
            by_split,
            PayeeUse {
                splits: 1,
                bill_plans: 0,
            },
        );
        factory.name_payee(
            &mut ports,
            by_plan,
            PayeeUse {
                splits: 0,
                bill_plans: 1,
            },
        );

        for id in [by_split, by_plan] {
            payees.payees_mut().delete_payee(id);
            factory.pending.commit();
            let failures = factory.failures();
            assert_eq!(failures.len(), 1, "the delete of {id} is refused");
            assert_eq!(failures[0].domains, [Domain::Payees]);
            payees.reload().expect("the rows load");
            assert!(payees.payees().payee(id).is_some(), "Payee {id} stays");
        }
    }

    /// A count the file can't answer is unknown, not unused, so the delete it would allow is
    /// refused before anything is written.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_count_that_fails_is_unknown_and_refuses_the_delete(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut payees = factory.create(default_payees());
        let netflix = PayeeId::seed(3);
        factory
            .writer
            .run(|pool| async move {
                sqlx::query("ALTER TABLE splits RENAME TO splits_gone")
                    .execute(&pool)
                    .await?;
                Ok(())
            })
            .expect("the table is renamed");

        assert_eq!(payees.usage().payee_usage(netflix), None);
        assert_eq!(
            lib_payees::interactor::DeletePayee(netflix).run(payees.split_mut()),
            Err(lib_payees::DeletePayeeRefusal::Unchecked)
        );
        assert!(payees.payees().payee(netflix).is_some(), "Netflix stays");
    }

    /// The new foreign keys refuse a Split or Bill Plan naming a Payee that doesn't exist.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_split_or_bill_plan_naming_no_stored_payee_is_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        let missing = Some(PayeeId::new());

        assert!(
            TransactionsAdapter::seed(&factory.writer, vec![transaction_naming(missing, 1)])
                .is_err()
        );
        assert!(
            BillsAdapter::seed(&factory.writer, vec![plan_naming(missing)], Vec::new()).is_err()
        );
        assert!(
            TransactionsAdapter::seed(&factory.writer, vec![transaction_naming(None, 1)]).is_ok(),
            "a Split may name no Payee"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn bills_read_the_payees_rows(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        let mut factory = AdapterFactory::new(options);
        let mut payees = factory.create(default_payees());
        let mut hudson = payees
            .payees()
            .payee(PayeeId::seed(17))
            .expect("a stub Payee");
        hudson.is_active = false;
        payees.payees_mut().upsert_payee(hudson);
        let payees = factory.reload(payees);

        let read = payees.bill_payees();
        assert_eq!(read.len(), default_payees().len());
        let hudson = read
            .iter()
            .find(|payee| payee.id == PayeeId::seed(17))
            .expect("every Payee, inactive ones included");
        assert_eq!(hudson.name, "Hudson News");
        assert!(!hudson.is_active);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_payees(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = PayeesAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.payees().payees().is_empty());
    }
}
