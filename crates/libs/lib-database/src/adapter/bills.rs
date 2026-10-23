//! The Bills adapter: `lib-bills`' Bill Plan and Bill Schedule ports over the `bill_plans` and
//! `bill_schedule_entries` tables, and the Match link on `splits` (ADR-0046, ADR-0019 as amended).
//!
//! It loads every row into memory, blocking, and serves reads from there. A port write changes
//! memory at once and joins the [`PendingWrites`] every adapter shares, which the bin commits
//! after each call, so Pay's Transaction and the entry it settles land together.
//!
//! An entry is written by its Bill Plan and due date, which the table keeps unique: a write for a
//! stored date updates that row and keeps its id. A Paid entry's link is the Split's
//! `bill_schedule_entry_id`, so writing an entry also moves that link, and reading one back maps
//! the Split that names it to `Resolution::Paid`. A Paid entry whose Split has since gone (deleted
//! with its Transaction) reads as Unresolved, as Unmatching it would leave it.

use chrono::NaiveDate;
use lib_bills::{
    AmountKind, BillPlan, BillPlanRepository, BillScheduleEntry, BillScheduleRepository,
    Recurrence, Resolution, SplitRef,
};
use lib_core::{
    AccountId, BillPlanId, BillScheduleEntryId, CategoryId, Money, PayeeId, TransactionId, UnitId,
};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum BillsChange {
    UpsertPlan(BillPlan),
    UpsertEntry(BillScheduleEntry),
}

/// The Bills domain, held in memory and saved through the shared writer.
pub struct BillsAdapter {
    plans: Vec<BillPlan>,
    entries: Vec<BillScheduleEntry>,
    pending: PendingWrites,
}

impl BillsAdapter {
    /// Loads every Bill Plan and entry, with each Paid entry's Split, through `pending`'s writer,
    /// blocking until they are read. The adapter's writes join `pending`. An empty database loads
    /// no rows.
    ///
    /// # Errors
    /// Returns the error of the read, or [`Error::Validation`] for a value the schema's checks
    /// should have kept out.
    #[tracing::instrument(name = "Load the Bills", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let (plans, entries) = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            let plans = read_plans(&mut conn).await?;
            let entries = read_entries(&mut conn).await?;
            Ok((plans, entries))
        })?;
        Ok(Self {
            plans,
            entries,
            pending,
        })
    }

    /// Writes `plans` and then `entries` in one transaction, blocking until it lands. Each is an
    /// upsert, so seeding again leaves the same rows. A Paid entry's Transaction must be stored
    /// first.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Bills", level = "debug", skip_all, err)]
    pub fn seed(
        writer: &Writer,
        plans: Vec<BillPlan>,
        entries: Vec<BillScheduleEntry>,
    ) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            for plan in &plans {
                upsert_plan(&mut conn, plan).await?;
            }
            for entry in &entries {
                upsert_entry(&mut conn, entry).await?;
            }
            conn.commit().await?;
            Ok(())
        })
    }

    /// Drops the Bills changes not yet committed and loads the rows again, blocking. For after a
    /// [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Bills`], so memory matches
    /// the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Bills", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.plans = fresh.plans;
        self.entries = fresh.entries;
        self.pending.discard(Domain::Bills);
        Ok(())
    }
}

impl BillPlanRepository for BillsAdapter {
    fn bill_plans(&self) -> Vec<BillPlan> {
        self.plans.clone()
    }

    fn upsert_bill_plan(&mut self, plan: BillPlan) {
        match self.plans.iter_mut().find(|row| row.id == plan.id) {
            Some(row) => *row = plan.clone(),
            None => self.plans.push(plan.clone()),
        }
        self.pending.push(BillsChange::UpsertPlan(plan));
    }
}

impl BillScheduleRepository for BillsAdapter {
    fn entries(&self) -> Vec<BillScheduleEntry> {
        self.entries.clone()
    }

    fn upsert_entry(&mut self, entry: BillScheduleEntry) {
        let entry = match self.entries.iter_mut().find(|row| row.id() == entry.id()) {
            Some(row) => {
                *row = BillScheduleEntry {
                    row_id: row.row_id,
                    ..entry
                };
                row.clone()
            }
            None => {
                self.entries.push(entry.clone());
                entry
            }
        };
        self.pending.push(BillsChange::UpsertEntry(entry));
    }
}

impl Change for BillsChange {
    fn domain(&self) -> Domain {
        Domain::Bills
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::UpsertPlan(plan) => upsert_plan(conn, &plan).await,
                Self::UpsertEntry(entry) => upsert_entry(conn, &entry).await,
            }
        })
    }
}

fn amount_kind_name(kind: AmountKind) -> &'static str {
    match kind {
        AmountKind::Fixed => "fixed",
        AmountKind::Estimated => "estimated",
    }
}

fn amount_kind_from(name: &str) -> Result<AmountKind> {
    match name {
        "fixed" => Ok(AmountKind::Fixed),
        "estimated" => Ok(AmountKind::Estimated),
        other => Err(Error::Validation(format!("unknown amount kind {other:?}"))),
    }
}

fn recurrence_name(recurrence: Recurrence) -> &'static str {
    match recurrence {
        Recurrence::Weekly => "weekly",
        Recurrence::Fortnightly => "fortnightly",
        Recurrence::Monthly => "monthly",
        Recurrence::Quarterly => "quarterly",
        Recurrence::Annually => "annually",
        Recurrence::OneShot => "one_shot",
    }
}

fn recurrence_from(name: &str) -> Result<Recurrence> {
    Recurrence::ALL
        .into_iter()
        .find(|recurrence| recurrence_name(*recurrence) == name)
        .ok_or_else(|| Error::Validation(format!("unknown recurrence {name:?}")))
}

async fn upsert_plan(conn: &mut SqliteConnection, plan: &BillPlan) -> Result<()> {
    let amount_kind = amount_kind_name(plan.amount_kind);
    let recurrence = recurrence_name(plan.recurrence);
    let attention_lead = plan.attention_lead.map(i64::from);
    sqlx::query!(
        "INSERT INTO bill_plans (id, name, category_id, unit_id, account_id, payee_id,
             planned_amount, amount_kind, recurrence, first_due, ends_on, attention_lead,
             is_active, generate_from)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             category_id = excluded.category_id,
             unit_id = excluded.unit_id,
             account_id = excluded.account_id,
             payee_id = excluded.payee_id,
             planned_amount = excluded.planned_amount,
             amount_kind = excluded.amount_kind,
             recurrence = excluded.recurrence,
             first_due = excluded.first_due,
             ends_on = excluded.ends_on,
             attention_lead = excluded.attention_lead,
             is_active = excluded.is_active,
             generate_from = excluded.generate_from",
        plan.id,
        plan.name,
        plan.category_id,
        plan.unit,
        plan.account_id,
        plan.payee_id,
        plan.planned_amount,
        amount_kind,
        recurrence,
        plan.first_due,
        plan.ends_on,
        attention_lead,
        plan.is_active,
        plan.generate_from,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Writes the entry by its Bill Plan and due date, then moves the Match link: off whichever Split
/// held it, and onto the Paid entry's Split.
async fn upsert_entry(conn: &mut SqliteConnection, entry: &BillScheduleEntry) -> Result<()> {
    let (resolution, skipped_planned) = match &entry.resolution {
        Resolution::Unresolved => ("unresolved", None),
        Resolution::Paid(_) => ("paid", None),
        Resolution::Skipped { planned } => ("skipped", Some(planned.clone())),
    };
    sqlx::query!(
        "INSERT INTO bill_schedule_entries
             (id, bill_plan_id, due, resolution, skipped_planned, superseded)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (bill_plan_id, due) DO UPDATE SET
             resolution = excluded.resolution,
             skipped_planned = excluded.skipped_planned,
             superseded = excluded.superseded",
        entry.row_id,
        entry.plan_id,
        entry.due,
        resolution,
        skipped_planned,
        entry.superseded,
    )
    .execute(&mut *conn)
    .await?;
    // The stored id, which a write for a date already stored keeps rather than `row_id`.
    let id = sqlx::query_scalar!(
        r#"SELECT id AS "id!: BillScheduleEntryId" FROM bill_schedule_entries
           WHERE bill_plan_id = ? AND due = ?"#,
        entry.plan_id,
        entry.due,
    )
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE splits SET bill_schedule_entry_id = NULL WHERE bill_schedule_entry_id = ?",
        id
    )
    .execute(&mut *conn)
    .await?;
    if let Resolution::Paid(split) = entry.resolution {
        let position = i64::try_from(split.split_index).map_err(|_| {
            Error::Validation(format!("Split {} is out of range", split.split_index))
        })?;
        let linked = sqlx::query!(
            "UPDATE splits SET bill_schedule_entry_id = ?
             WHERE transaction_id = ? AND position = ?",
            id,
            split.transaction_id,
            position,
        )
        .execute(&mut *conn)
        .await?;
        // The Match's foreign key: a Paid entry names a stored Split.
        if linked.rows_affected() != 1 {
            return Err(Error::Validation(format!(
                "Bill Schedule entry {id} is paid by Split {position} of Transaction {}, which \
                 isn't stored",
                split.transaction_id
            )));
        }
    }
    Ok(())
}

async fn read_plans(conn: &mut SqliteConnection) -> Result<Vec<BillPlan>> {
    sqlx::query!(
        r#"SELECT
               id             AS "id!: BillPlanId",
               name,
               category_id    AS "category_id!: CategoryId",
               unit_id        AS "unit_id!: UnitId",
               account_id     AS "account_id!: AccountId",
               payee_id       AS "payee_id?: PayeeId",
               planned_amount AS "planned_amount!: Money",
               amount_kind,
               recurrence,
               first_due      AS "first_due!: NaiveDate",
               ends_on        AS "ends_on?: NaiveDate",
               attention_lead,
               is_active,
               generate_from  AS "generate_from!: NaiveDate"
           FROM bill_plans
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        Ok(BillPlan {
            id: row.id,
            name: row.name,
            category_id: row.category_id,
            unit: row.unit_id,
            account_id: row.account_id,
            payee_id: row.payee_id,
            planned_amount: row.planned_amount,
            amount_kind: amount_kind_from(&row.amount_kind)?,
            recurrence: recurrence_from(&row.recurrence)?,
            first_due: row.first_due,
            ends_on: row.ends_on,
            attention_lead: row
                .attention_lead
                .map(u32::try_from)
                .transpose()
                .map_err(|_| {
                    Error::Validation(format!("Bill Plan {}'s lead is out of range", row.id))
                })?,
            is_active: row.is_active,
            generate_from: row.generate_from,
        })
    })
    .collect()
}

async fn read_entries(conn: &mut SqliteConnection) -> Result<Vec<BillScheduleEntry>> {
    sqlx::query!(
        r#"SELECT
               entry.id              AS "id!: BillScheduleEntryId",
               entry.bill_plan_id    AS "plan_id!: BillPlanId",
               entry.due             AS "due!: NaiveDate",
               entry.resolution,
               entry.skipped_planned AS "skipped_planned?: Money",
               entry.superseded,
               split.transaction_id  AS "transaction_id?: TransactionId",
               split.position        AS "position?: i64"
           FROM bill_schedule_entries AS entry
           LEFT JOIN splits AS split ON split.bill_schedule_entry_id = entry.id
           ORDER BY entry.due, entry.bill_plan_id"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        let paid_by =
            row.transaction_id
                .zip(row.position)
                .and_then(|(transaction_id, position)| {
                    Some(SplitRef {
                        transaction_id,
                        split_index: usize::try_from(position).ok()?,
                    })
                });
        let resolution = match (row.resolution.as_str(), paid_by, row.skipped_planned) {
            ("paid", Some(split), _) => Resolution::Paid(split),
            // Its Split was deleted, which Unmatches it (ADR-0019).
            ("paid", None, _) | ("unresolved", _, _) => Resolution::Unresolved,
            ("skipped", _, Some(planned)) => Resolution::Skipped { planned },
            (other, _, _) => {
                return Err(Error::Validation(format!(
                    "Bill Schedule entry {} has resolution {other:?}",
                    row.id
                )));
            }
        };
        Ok(BillScheduleEntry {
            row_id: row.id,
            plan_id: row.plan_id,
            due: row.due,
            resolution,
            superseded: row.superseded,
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use lib_bills::BillSplit;
    use lib_bills::contract::{self, PortsFactory};
    use lib_bills::fake::InMemoryBills;
    use lib_core::TransactionStatus;
    use lib_transactions::{Split, Transaction, TransactionRepository};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{TransactionsAdapter, UnitsAdapter};

    /// The adapter on one test database, with its pending batch. The stub's Units are stored first,
    /// which a Bill Plan's Unit names. A reload commits, loads afresh, and fails the check if the
    /// writer reported a failure, so a write SQLite refuses can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            UnitsAdapter::seed_from(
                &writer,
                lib_units::fake::InMemoryPorts::stub(chrono::Utc::now()),
            )
            .expect("the stub Units are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        fn transactions(&self) -> TransactionsAdapter {
            TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load")
        }

        /// Makes every write to `table` fail from now on.
        fn refuse(&self, table: &'static str) {
            self.writer
                .run(move |pool| async move {
                    sqlx::query(&format!(
                        "CREATE TRIGGER refuse_{table} BEFORE INSERT ON {table}
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END"
                    ))
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }
    }

    /// `splits` grouped by Transaction into the Transactions they are Splits of, Open, unflagged
    /// and untagged, each Split at its index: what the Transactions store holds for Bills to read.
    fn transactions_of(mut splits: Vec<BillSplit>) -> Vec<Transaction> {
        splits.sort_by_key(|s| (s.split.transaction_id, s.split.split_index));
        let mut transactions: Vec<Transaction> = Vec::new();
        for s in splits {
            let split = Split {
                amount: s.amount,
                category_id: s.category_id,
                payee_id: s.payee_id,
                tag_ids: Vec::new(),
            };
            match transactions.last_mut() {
                Some(t) if t.id == s.split.transaction_id => t.splits.push(split),
                _ => transactions.push(Transaction {
                    id: s.split.transaction_id,
                    date: s.date,
                    account_id: s.account_id,
                    status: TransactionStatus::Open,
                    is_flagged: false,
                    description: s.description,
                    splits: vec![split],
                }),
            }
        }
        transactions
    }

    impl PortsFactory for AdapterFactory {
        type Ports = BillsAdapter;
        type Reads = TransactionsAdapter;

        fn create(&mut self, seed: InMemoryBills, splits: Vec<BillSplit>) -> BillsAdapter {
            TransactionsAdapter::seed(&self.writer, transactions_of(splits))
                .expect("the Transactions are written");
            BillsAdapter::seed(&self.writer, seed.plans, seed.entries)
                .expect("the seed is written");
            BillsAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reads(&mut self) -> TransactionsAdapter {
            self.transactions()
        }

        fn reload(&mut self, _ports: BillsAdapter) -> BillsAdapter {
            self.pending.commit();
            let fresh = BillsAdapter::load(self.pending.clone()).expect("the rows load");
            if let Ok(failure) = self.failures.try_recv() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut BillsAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut BillsAdapter) {
            self.refuse("bill_plans");
        }

        fn recover(&mut self, mut ports: BillsAdapter) -> (BillsAdapter, usize) {
            self.writer.flush().expect("the writer is running");
            let mut failures = 0;
            while let Ok(failure) = self.failures.try_recv() {
                if failure.domains == [Domain::Bills] {
                    failures += 1;
                }
            }
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn plans_and_entries_round_trip(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::plans_and_entries_round_trip(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_entry_is_found_by_its_plan_and_due_date(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_entry_is_found_by_its_plan_and_due_date(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_paid_link_round_trips(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::the_paid_link_round_trips(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_splits_read_back_as_they_were_stored(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_splits_read_back_as_they_were_stored(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// The stub Ledger's Bills and the Splits their Paid entries name, as of a fixed day.
    fn stub() -> (Vec<BillSplit>, InMemoryBills) {
        let ledger =
            lib_ledger::Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date"));
        (
            ledger.bill_splits(),
            InMemoryBills {
                plans: ledger.bill_plans().to_vec(),
                entries: ledger.bill_entries().to_vec(),
            },
        )
    }

    fn paid_entry(bills: &BillsAdapter) -> BillScheduleEntry {
        bills
            .entries()
            .into_iter()
            .find(|entry| matches!(entry.resolution, Resolution::Paid(_)))
            .expect("the stub pays an entry")
    }

    fn paid_by(entry: &BillScheduleEntry) -> TransactionId {
        match entry.resolution {
            Resolution::Paid(split) => split.transaction_id,
            _ => panic!("the entry is Paid"),
        }
    }

    /// The Match is the Split's, so a Paid entry whose Split isn't stored is refused.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_paid_entry_naming_no_stored_split_is_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let (splits, seed) = stub();
        let mut bills = factory.create(seed, splits);
        let open = bills
            .entries()
            .into_iter()
            .find(BillScheduleEntry::is_open)
            .expect("the stub leaves an entry open");

        bills.upsert_entry(BillScheduleEntry {
            resolution: Resolution::Paid(SplitRef {
                transaction_id: TransactionId::new(),
                split_index: 0,
            }),
            ..open
        });
        factory.pending.commit();
        factory.writer.flush().expect("the writer is running");

        let failure = factory.failures.try_recv().expect("the write failed");
        assert_eq!(failure.domains, [Domain::Bills]);
        assert!(matches!(failure.error, Error::Validation(_)));
    }

    /// Deleting a Paid entry's Transaction deletes its Splits, and with them the Match, so the
    /// entry loads as Unmatched; rewriting the Transaction, as a Tag merge does, keeps it.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_match_follows_its_transaction(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let (splits, seed) = stub();
        let bills = factory.create(seed, splits);
        let mut ledger_transactions = factory.transactions();
        let rewritten = paid_entry(&bills);
        let mut transaction = ledger_transactions
            .transaction(paid_by(&rewritten))
            .expect("the paying Transaction is stored");
        transaction.description = Some("Edited".into());

        ledger_transactions.upsert_transaction(transaction);
        let bills = factory.reload(bills);
        assert_eq!(bills.entry(rewritten.id()), Some(rewritten.clone()));

        ledger_transactions.delete_transaction(paid_by(&rewritten));
        let bills = factory.reload(bills);
        assert_eq!(
            bills.entry(rewritten.id()),
            Some(BillScheduleEntry {
                resolution: Resolution::Unresolved,
                ..rewritten
            })
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_bills(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = BillsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.bill_plans().is_empty());
        assert!(adapter.entries().is_empty());
    }
}
