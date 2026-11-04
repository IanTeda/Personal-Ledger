//! The Payees adapter: `lib-payees`' Payee repository port over the `payees` and `payee_aliases`
//! tables, and `lib-bills`' `BillPayees` and `lib-transactions`' `TransactionPayees` read ports
//! from the same rows (ADR-0046, #925).
//!
//! It loads every Payee and its aliases into memory, blocking, and serves the rows from there. A
//! port write changes memory at once and joins the [`PendingWrites`] every adapter shares, which
//! the bin commits after each input. Writing a Payee rewrites its aliases, in the order they are
//! held, so they read back in that order; deleting one takes its aliases with it (the schema's
//! cascade).
//!
//! Usage is other domains' rows, so `lib-payees`' `PayeeUsage` is answered by
//! [`StoredPayeeReads`](crate::StoredPayeeReads) from the Transactions and Bills stores (#992),
//! counted in one read of the file they share ([`payee_usage`]). A Payee those tables name can't be deleted (their foreign keys), which
//! backstops the delete refusal.

use lib_bills::BillPayee;
use lib_core::{CategoryId, PayeeId, RowID};
use lib_payees::{Payee, PayeeRepository, PayeeUse};
use lib_transactions::TransactionPayee;
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
}

/// The rows, with the batch their writes join.
struct StoredPayees {
    rows: Vec<Payee>,
    pending: PendingWrites,
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
            Self::seed_in(&mut conn, &payees).await?;
            conn.commit().await?;
            Ok(())
        })
    }

    /// [`Self::seed`]'s writes on `conn`, so a seed of several domains commits them in one
    /// transaction (#886).
    ///
    /// # Errors
    /// Returns the error of a write.
    pub(crate) async fn seed_in(conn: &mut SqliteConnection, payees: &[Payee]) -> Result<()> {
        for payee in payees {
            upsert(conn, payee).await?;
        }
        Ok(())
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

/// The Splits and Bill Plans naming Payee `id` in the file, counted in one job on `writer`, one
/// blocking round trip rather than two: what [`StoredPayeeReads`](crate::StoredPayeeReads) answers
/// when the Transactions and Bills stores share the file. `None` when it can't be read, logged, so a
/// Payee that may be in use is never offered for deleting (#840).
pub(crate) fn payee_usage(writer: &Writer, id: PayeeId) -> Option<PayeeUse> {
    writer
        .run(move |pool| async move {
            let counts = sqlx::query!(
                r#"SELECT
                    (SELECT COUNT(*) FROM splits WHERE payee_id = ?) AS "splits!: i64",
                    (SELECT COUNT(*) FROM bill_plans WHERE payee_id = ?) AS "plans!: i64""#,
                id,
                id
            )
            .fetch_one(&pool)
            .await?;
            let count = |count: i64, what: &str| {
                usize::try_from(count)
                    .map_err(|_| Error::Validation(format!("Payee {id}'s {what} count is {count}")))
            };
            Ok(PayeeUse {
                splits: count(counts.splits, "Splits")?,
                bill_plans: count(counts.plans, "Bill Plans")?,
            })
        })
        .inspect_err(|error| {
            tracing::error!(%error, payee = %id, "Failed to count what names a Payee");
        })
        .ok()
}

/// `payees` as `lib-documents`' `DocumentPayees` reads them, inactive ones included.
pub(crate) fn document_payees(payees: &[Payee]) -> Vec<lib_documents::DocumentPayee> {
    payees
        .iter()
        .map(|payee| lib_documents::DocumentPayee {
            id: payee.id,
            name: payee.name.clone(),
            aliases: payee.aliases.clone(),
            is_active: payee.is_active,
        })
        .collect()
}

impl Change for PayeesChange {
    fn domain(&self) -> Domain {
        Domain::Payees
    }

    /// A deleted Payee drops the Document Links to it.
    fn cascades_to(&self) -> Option<Domain> {
        matches!(self, Self::Delete(_)).then_some(Domain::Documents)
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

/// `payees` as Transactions reads them.
pub(crate) fn transaction_payees(payees: &[Payee]) -> Vec<TransactionPayee> {
    payees
        .iter()
        .map(|payee| TransactionPayee {
            id: payee.id,
            name: payee.name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_bills::{AmountKind, BillPlan, Recurrence};
    use lib_core::{AccountId, BillPlanId, Money, TransactionId, TransactionStatus, UnitId};
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_payees::contract::{self, PortsFactory, StoredPorts};
    use lib_payees::{PayeeUsage, PayeeUse, PortsMut, default_payees};
    use lib_transactions::{Split, Transaction};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::test_support::TransactionReadsFactory;
    use crate::writer::WriteFailure;
    use crate::{BillsAdapter, CategoriesAdapter, StoredPayeeReads, TransactionsAdapter};

    /// The Payees adapter with its usage read as a Client reads it: through [`StoredPayeeReads`]
    /// over the Transactions, Bills and Categories adapters on the same file.
    struct StoredPayeesPorts {
        payees: PayeesAdapter,
        reads: StoredPayeeReads,
    }

    impl StoredPorts for StoredPayeesPorts {
        fn split_mut(&mut self) -> PortsMut<'_> {
            PortsMut {
                payees: self.payees.payees_mut(),
                usage: &self.reads,
            }
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

        /// The Payees as stored, with the reads over the other adapters on the same file.
        fn load(&self) -> StoredPayeesPorts {
            let load = || {
                Ok::<_, Error>(StoredPayeeReads {
                    transactions: Rc::new(RefCell::new(TransactionsAdapter::load(
                        self.pending.clone(),
                    )?)),
                    bills: Rc::new(RefCell::new(BillsAdapter::load(self.pending.clone())?)),
                    categories: Rc::new(RefCell::new(CategoriesAdapter::load(
                        self.pending.clone(),
                    )?)),
                })
            };
            StoredPayeesPorts {
                payees: PayeesAdapter::load(self.pending.clone()).expect("the rows load"),
                reads: load().expect("the other domains' rows load"),
            }
        }

        /// Settles the writes sent so far and returns the failures reported since the last call.
        fn failures(&self) -> Vec<WriteFailure> {
            self.writer.flush().expect("the writer is running");
            self.failures.try_iter().collect()
        }
    }

    /// `lib-transactions`' read check: each Payee is stored active, with no aliases or default.
    #[sqlx::test(migrations = "migrations/client")]
    async fn transactions_read_the_payees_rows(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        fn store(writer: &Writer, rows: Vec<TransactionPayee>) {
            let payees = rows
                .into_iter()
                .map(|row| Payee {
                    id: row.id,
                    name: row.name,
                    aliases: Vec::new(),
                    default_category: None,
                    is_active: true,
                })
                .collect();
            PayeesAdapter::seed(writer, payees).expect("the Payees are written");
        }
        lib_transactions::contract::payees_read_back(&mut TransactionReadsFactory::new(
            options,
            store,
            crate::test_support::transaction_reads,
        ));
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
        type Ports = StoredPayeesPorts;

        fn create(&mut self, seed: Vec<Payee>) -> StoredPayeesPorts {
            PayeesAdapter::seed(&self.writer, seed).expect("the seed is written");
            self.load()
        }

        fn reload(&mut self, _ports: StoredPayeesPorts) -> StoredPayeesPorts {
            self.pending.commit();
            let fresh = self.load();
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut StoredPayeesPorts) {
            self.pending.commit();
        }

        fn name_payee(&mut self, _ports: &mut StoredPayeesPorts, id: PayeeId, uses: PayeeUse) {
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

        fn break_writes(&mut self, _ports: &mut StoredPayeesPorts) {
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

        fn recover(&mut self, mut ports: StoredPayeesPorts) -> (StoredPayeesPorts, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Payees])
                .count();
            ports.payees.reload().expect("the rows load");
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
        let mut ports = factory.create(default_payees());
        let (by_split, by_plan) = (PayeeId::seed(1), PayeeId::seed(2));
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

        let payees = &mut ports.payees;
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

    /// A count the file can't answer, of either half, is unknown, not unused, so the delete it
    /// would allow is refused before anything is written.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_count_that_fails_is_unknown_and_refuses_the_delete(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut ports = factory.create(default_payees());
        let netflix = PayeeId::seed(3);
        let rename = |table: &'static str| {
            factory
                .writer
                .run(move |pool| async move {
                    sqlx::query(&format!("ALTER TABLE {table} RENAME TO {table}_gone"))
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the table is renamed");
        };

        rename("bill_plans");
        assert_eq!(
            ports.reads.payee_usage(netflix),
            None,
            "no Bill Plans count"
        );
        rename("splits");
        assert_eq!(ports.reads.payee_usage(netflix), None);
        assert_eq!(
            lib_payees::interactor::DeletePayee(netflix).run(ports.split_mut()),
            Err(lib_payees::DeletePayeeRefusal::Unchecked)
        );
        assert!(
            ports.payees.payees().payee(netflix).is_some(),
            "Netflix stays"
        );
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
        let mut ports = factory.create(default_payees());
        let mut hudson = ports
            .payees
            .payees()
            .payee(PayeeId::seed(17))
            .expect("a stub Payee");
        hudson.is_active = false;
        ports.payees.payees_mut().upsert_payee(hudson);
        let ports = factory.reload(ports);

        let read = bill_payees(&ports.payees.payees.rows);
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
