//! The Accounts adapter: `lib-accounts`' repository and `AccountUsage` ports over the `accounts`
//! table, and the reads other modules make of the same rows: `lib-units`' `UnitHoldings` and the
//! Accounts half of its `UnitUsage`, and `lib-bills`' `BillAccounts` (ADR-0046, #853).
//!
//! It loads every Account into memory, blocking, and serves the rows from there, in the order they
//! were added. A port write changes memory at once and joins the [`PendingWrites`] every adapter
//! shares, which the bin commits after each input. Every Account names a stored Institution, a
//! Cash Account usually the Personal Institution (ADR-0049), so `institution_id` is never NULL.
//!
//! Usage is other domains' rows, so [`AccountUsage`] counts the `transactions` booked to an
//! Account in the file, blocking: what the last commit left, not a write still pending. Budgets
//! aren't persisted, so its Budgets count is always 0 here; the Desktop adds the `Ledger`'s until
//! Budgets persist (#811). The count is only a figure the Account Dialogs and the Units page show,
//! so one that can't be read is logged and answered as none. The `transactions` and `bill_plans`
//! foreign keys refuse deleting an Account they name.

use std::collections::HashMap;

use chrono::NaiveDate;
use lib_accounts::{Account, AccountRepository, AccountUsage, AccountUse};
use lib_bills::{BillAccount, BillAccounts};
use lib_core::{AccountId, AccountType, InstitutionId, Money, UnitId};
use lib_units::{Holding, UnitHoldings, UnitUsage, UnitUse};
use sqlx::SqliteConnection;

use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};
use crate::{Error, Result};

/// One port write, saved in the order it was made.
#[derive(Debug, Clone)]
enum AccountsChange {
    Upsert(Account),
    Delete(AccountId),
}

/// The Accounts domain, held in memory and saved through the shared writer.
pub struct AccountsAdapter {
    accounts: StoredAccounts,
    usage: StoredUsage,
}

/// The rows, with the batch their writes join.
struct StoredAccounts {
    rows: Vec<Account>,
    pending: PendingWrites,
}

/// Counts in the file, read through the writer.
struct StoredUsage {
    writer: Writer,
}

impl AccountsAdapter {
    /// Loads every Account through `pending`'s writer, blocking until they are read. The adapter's
    /// writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Accounts", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self {
            usage: StoredUsage {
                writer: pending.writer().clone(),
            },
            accounts: StoredAccounts { rows, pending },
        })
    }

    /// Writes `accounts` in one transaction, blocking until it lands. Each is an upsert, so
    /// seeding again leaves the same rows. For after the Units and Institutions they name, and
    /// before the Transactions and Bill Plans that name them.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Accounts", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, accounts: Vec<Account>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            for account in &accounts {
                upsert(&mut conn, account).await?;
            }
            conn.commit().await?;
            Ok(())
        })
    }

    /// Drops the Accounts changes not yet committed and loads the rows again, blocking. For after
    /// a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Accounts`], so memory
    /// matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Accounts", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.accounts.pending.clone())?;
        self.accounts.rows = fresh.accounts.rows;
        self.accounts.pending.discard(Domain::Accounts);
        Ok(())
    }

    pub fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    pub fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    pub fn usage(&self) -> &dyn AccountUsage {
        &self.usage
    }
}

impl AccountRepository for StoredAccounts {
    fn accounts(&self) -> Vec<Account> {
        self.rows.clone()
    }

    fn upsert_account(&mut self, account: Account) {
        match self.rows.iter_mut().find(|row| row.id == account.id) {
            Some(row) => *row = account.clone(),
            None => self.rows.push(account.clone()),
        }
        self.pending.push(AccountsChange::Upsert(account));
    }

    fn delete_account(&mut self, id: AccountId) {
        self.rows.retain(|row| row.id != id);
        self.pending.push(AccountsChange::Delete(id));
    }
}

impl StoredUsage {
    /// How many Transactions are booked to each Account that has any. A count that can't be read
    /// is logged and answered as none, since it only feeds a figure.
    fn transaction_counts(&self) -> HashMap<AccountId, usize> {
        self.writer
            .run(|pool| async move {
                let mut conn = pool.acquire().await?;
                count_transactions(&mut conn).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, "Failed to count the Transactions booked to each Account");
            })
            .unwrap_or_default()
    }
}

impl AccountUsage for StoredUsage {
    /// The Budgets count is the `Ledger`'s until Budgets persist (#811), so it is 0 here.
    fn account_usage(&self, id: AccountId) -> AccountUse {
        let transactions = self
            .writer
            .run(move |pool| async move {
                let mut conn = pool.acquire().await?;
                count_transactions_of(&mut conn, id).await
            })
            .inspect_err(|error| {
                tracing::error!(%error, account = %id, "Failed to count an Account's Transactions");
            })
            .unwrap_or_default();
        AccountUse {
            transactions,
            budgets: 0,
        }
    }
}

impl UnitHoldings for AccountsAdapter {
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        let counts = self.usage.transaction_counts();
        holdings(&self.accounts.rows, id, |account| {
            counts.get(&account).copied().unwrap_or_default()
        })
    }
}

/// Only the Accounts half: the Bill Plans and Budgets naming a Unit are other domains' to count.
impl UnitUsage for AccountsAdapter {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        UnitUse {
            accounts: unit_accounts(&self.accounts.rows, id),
            ..UnitUse::default()
        }
    }
}

impl BillAccounts for AccountsAdapter {
    fn bill_accounts(&self) -> Vec<BillAccount> {
        bill_accounts(&self.accounts.rows)
    }
}

/// The Accounts in `accounts` whose Unit is `unit`, in order, as the Units page lists them, each
/// with the Transactions `transactions` says are booked to it.
pub(crate) fn holdings(
    accounts: &[Account],
    unit: UnitId,
    transactions: impl Fn(AccountId) -> usize,
) -> Vec<Holding> {
    accounts
        .iter()
        .filter(|account| account.unit == unit)
        .map(|account| Holding {
            name: account.name.clone(),
            kind: account.account_type.clone(),
            balance: account.balance.clone(),
            transactions: u32::try_from(transactions(account.id)).unwrap_or(u32::MAX),
        })
        .collect()
}

/// How many of `accounts` count in `unit`.
pub(crate) fn unit_accounts(accounts: &[Account], unit: UnitId) -> usize {
    accounts
        .iter()
        .filter(|account| account.unit == unit)
        .count()
}

/// `accounts` as Bills reads them, each flagged by `lib-transactions`' rule for whether it takes
/// Transactions.
pub(crate) fn bill_accounts(accounts: &[Account]) -> Vec<BillAccount> {
    accounts
        .iter()
        .map(|account| BillAccount {
            id: account.id,
            name: account.name.clone(),
            unit: account.unit,
            takes_transactions: lib_transactions::takes_transactions(&account.account_type),
        })
        .collect()
}

impl Change for AccountsChange {
    fn domain(&self) -> Domain {
        Domain::Accounts
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(account) => upsert(conn, &account).await,
                Self::Delete(id) => delete(conn, id).await,
            }
        })
    }
}

async fn upsert(conn: &mut SqliteConnection, account: &Account) -> Result<()> {
    sqlx::query!(
        "INSERT INTO accounts
             (id, name, institution_id, account_type, unit_id, balance, opened_at, account_number)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             institution_id = excluded.institution_id,
             account_type = excluded.account_type,
             unit_id = excluded.unit_id,
             balance = excluded.balance,
             opened_at = excluded.opened_at,
             account_number = excluded.account_number",
        account.id,
        account.name,
        account.institution,
        account.account_type,
        account.unit,
        account.balance,
        account.opened_at,
        account.account_number,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn delete(conn: &mut SqliteConnection, id: AccountId) -> Result<()> {
    sqlx::query!("DELETE FROM accounts WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn count_transactions_of(conn: &mut SqliteConnection, id: AccountId) -> Result<usize> {
    let count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM transactions WHERE account_id = ?"#,
        id,
    )
    .fetch_one(&mut *conn)
    .await?;
    usize::try_from(count)
        .map_err(|_| Error::Validation(format!("Account {id}'s Transactions count is {count}")))
}

async fn count_transactions(conn: &mut SqliteConnection) -> Result<HashMap<AccountId, usize>> {
    sqlx::query!(
        r#"SELECT account_id AS "account_id!: AccountId", COUNT(*) AS "count!: i64"
           FROM transactions
           GROUP BY account_id"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| {
        usize::try_from(row.count)
            .map(|count| (row.account_id, count))
            .map_err(|_| {
                Error::Validation(format!(
                    "Account {}'s Transactions count is {}",
                    row.account_id, row.count
                ))
            })
    })
    .collect()
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Account>> {
    let rows = sqlx::query!(
        r#"SELECT
               id             AS "id!: AccountId",
               name,
               institution_id AS "institution_id!: InstitutionId",
               account_type   AS "account_type!: AccountType",
               unit_id        AS "unit_id!: UnitId",
               balance        AS "balance!: Money",
               opened_at      AS "opened_at!: NaiveDate",
               account_number
           FROM accounts
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Account {
        id: row.id,
        name: row.name,
        institution: row.institution_id,
        account_type: row.account_type,
        unit: row.unit_id,
        balance: row.balance,
        account_number: row.account_number,
        opened_at: row.opened_at,
    })
    .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bigdecimal::BigDecimal;
    use lib_accounts::contract::{self, PortsFactory, StoredPorts};
    use lib_accounts::default_accounts;
    use lib_accounts::fake::InMemoryAccountUsage;
    use lib_bills::{AmountKind, BillPlan, Recurrence};
    use lib_core::{BillPlanId, CategoryId, TransactionId, TransactionStatus};
    use lib_transactions::{Split, Transaction};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{BillsAdapter, InstitutionsAdapter, TransactionsAdapter, UnitsAdapter};

    /// The adapter with the Budgets naming each Account kept beside it, as the Desktop adds the
    /// `Ledger`'s Budgets count to the adapter's Transactions count until Budgets persist (#811).
    struct AdapterPorts {
        adapter: AccountsAdapter,
        budgets: InMemoryAccountUsage,
    }

    impl AccountUsage for AdapterPorts {
        fn account_usage(&self, id: AccountId) -> AccountUse {
            AccountUse {
                budgets: self.budgets.account_usage(id).budgets,
                ..self.adapter.usage().account_usage(id)
            }
        }
    }

    impl StoredPorts for AdapterPorts {
        fn accounts(&mut self) -> &mut dyn AccountRepository {
            self.adapter.accounts_mut()
        }

        fn usage(&self) -> &dyn AccountUsage {
            self
        }
    }

    /// The adapter on one test database, with its pending batch. The stub's Units,
    /// Institutions and Categories are stored first, which the stub Accounts and a test's Splits
    /// and Bill Plans name. A reload commits, loads
    /// afresh, and fails the check if the writer reported a failure, so a write SQLite refuses
    /// can't pass unseen.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
        budgets: InMemoryAccountUsage,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            UnitsAdapter::seed_from(
                &writer,
                lib_units::fake::InMemoryPorts::stub(chrono::Utc::now()),
            )
            .expect("the stub Units are written");
            InstitutionsAdapter::seed(&writer, crate::seed::stub_institutions())
                .expect("the stub Institutions are written");
            crate::CategoriesAdapter::seed(&writer, crate::seed::stub_categories())
                .expect("the stub Categories are written");
            crate::TagsAdapter::seed(&writer, crate::seed::stub_tags())
                .expect("the stub Tags are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
                budgets: InMemoryAccountUsage::default(),
            }
        }

        fn ports(&self) -> AdapterPorts {
            AdapterPorts {
                adapter: AccountsAdapter::load(self.pending.clone()).expect("the rows load"),
                budgets: self.budgets.clone(),
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

    /// A Transaction of one Split booked to `account`.
    fn transaction_on(account: AccountId) -> Transaction {
        Transaction {
            id: TransactionId::new(),
            date: day(),
            account_id: account,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: vec![Split {
                amount: Money(BigDecimal::from(-10)),
                category_id: CategoryId::seed(7),
                payee_id: None,
                tag_ids: Vec::new(),
            }],
        }
    }

    fn plan_on(account: AccountId) -> BillPlan {
        BillPlan {
            id: BillPlanId::new(),
            name: "Plan".into(),
            category_id: CategoryId::seed(4),
            unit: UnitId::SEED_AUD,
            account_id: account,
            payee_id: None,
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
        type Ports = AdapterPorts;

        fn create(&mut self, seed: Vec<Account>) -> AdapterPorts {
            AccountsAdapter::seed(&self.writer, seed).expect("the seed is written");
            self.ports()
        }

        fn reload(&mut self, _ports: AdapterPorts) -> AdapterPorts {
            self.pending.commit();
            let fresh = self.ports();
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut AdapterPorts) {
            self.pending.commit();
        }

        fn name_account(&mut self, ports: &mut AdapterPorts, id: AccountId, uses: AccountUse) {
            let transactions = (0..uses.transactions).map(|_| transaction_on(id)).collect();
            TransactionsAdapter::seed(&self.writer, transactions)
                .expect("the Transactions are written");
            for counted in [&mut self.budgets, &mut ports.budgets] {
                counted.uses.entry(id).or_default().budgets += uses.budgets;
            }
        }

        fn break_writes(&mut self, _ports: &mut AdapterPorts) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_accounts BEFORE INSERT ON accounts
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: AdapterPorts) -> (AdapterPorts, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Accounts])
                .count();
            ports.adapter.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_added_account_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_added_account_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_edited_account_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_edited_account_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_account_stays_deleted(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_deleted_account_stays_deleted(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn accounts_list_in_the_order_added(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::accounts_list_in_the_order_added(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn usage_counts_the_rows_naming_an_account(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::usage_counts_the_rows_naming_an_account(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// A Cash Account names a stored Institution like any other, here the Personal Institution.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_cash_account_round_trips_its_institution(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        factory.create(default_accounts());
        let wallet = AccountId::seed(1);

        let reloaded = factory.ports().adapter.accounts().account(wallet);
        assert_eq!(
            reloaded.map(|a| (a.account_type, a.institution)),
            Some((AccountType::Cash, InstitutionId::SEED_PERSONAL))
        );
    }

    /// `institution_id` is `NOT NULL`: no Account, Cash included, is stored without an Institution.
    #[sqlx::test(migrations = "migrations/client")]
    async fn an_account_with_no_institution_is_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);

        let refused = factory.writer.run(|pool| async move {
            sqlx::query(
                "INSERT INTO accounts (id, name, institution_id, account_type, unit_id, balance, opened_at)
                 VALUES (?, 'Tin', NULL, 'cash', ?, '0', '2026-01-01')",
            )
            .bind(AccountId::new())
            .bind(UnitId::SEED_AUD)
            .execute(&pool)
            .await?;
            Ok(())
        });
        assert!(refused.is_err());
    }

    /// The new foreign keys refuse an Account naming an Institution that isn't stored, and a
    /// Transaction or Bill Plan naming an Account that isn't.
    #[sqlx::test(migrations = "migrations/client")]
    async fn rows_naming_no_stored_account_or_institution_are_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        factory.create(default_accounts());
        let missing = AccountId::new();
        let mut orphan = default_accounts()[1].clone();
        orphan.id = AccountId::new();
        orphan.institution = InstitutionId::new();

        assert!(AccountsAdapter::seed(&factory.writer, vec![orphan]).is_err());
        assert!(TransactionsAdapter::seed(&factory.writer, vec![transaction_on(missing)]).is_err());
        assert!(BillsAdapter::seed(&factory.writer, vec![plan_on(missing)], Vec::new()).is_err());
        assert!(
            TransactionsAdapter::seed(&factory.writer, vec![transaction_on(AccountId::seed(2))])
                .is_ok(),
            "a stored Account takes a Transaction"
        );
    }

    /// Deleting an Account a Transaction or Bill Plan names fails in the file and the Account
    /// stays; its Balance Checks go with it when nothing else names it.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_deleting_a_named_account_and_cascades_balance_checks(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut ports = factory.create(default_accounts());
        let (by_transaction, by_plan, checked) =
            (AccountId::seed(2), AccountId::seed(4), AccountId::seed(3));
        TransactionsAdapter::seed(&factory.writer, vec![transaction_on(by_transaction)])
            .expect("the Transaction is written");
        BillsAdapter::seed(&factory.writer, vec![plan_on(by_plan)], Vec::new())
            .expect("the Bill Plan is written");

        for id in [by_transaction, by_plan] {
            ports.adapter.accounts_mut().delete_account(id);
            factory.pending.commit();
            let failures = factory.failures();
            assert_eq!(failures.len(), 1, "the delete of {id} is refused");
            assert_eq!(failures[0].domains, [Domain::Accounts]);
            ports.adapter.reload().expect("the rows load");
            assert!(ports.adapter.accounts().account(id).is_some(), "{id} stays");
        }

        let check_count = |writer: &Writer| {
            writer
                .run(move |pool| async move {
                    let count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM balance_checks WHERE account_id = ?",
                    )
                    .bind(checked)
                    .fetch_one(&pool)
                    .await?;
                    Ok(count)
                })
                .expect("the count reads")
        };
        factory
            .writer
            .run(move |pool| async move {
                sqlx::query(
                    "INSERT INTO balance_checks (id, account_id, date, asserted_balance)
                     VALUES (?, ?, '2026-09-01', '100')",
                )
                .bind(lib_core::RowID::new())
                .bind(checked)
                .execute(&pool)
                .await?;
                Ok(())
            })
            .expect("the Balance Check is written");
        assert_eq!(check_count(&factory.writer), 1);

        ports.adapter.accounts_mut().delete_account(checked);
        let ports = factory.reload(ports);

        assert!(ports.adapter.accounts().account(checked).is_none());
        assert_eq!(
            check_count(&factory.writer),
            0,
            "its Balance Check went with it"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn units_and_bills_read_the_accounts_rows(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut ports = factory.create(default_accounts());
        let everyday = AccountId::seed(2);
        factory.name_account(
            &mut ports,
            everyday,
            AccountUse {
                transactions: 2,
                budgets: 0,
            },
        );
        let adapter = &ports.adapter;

        let aud: Vec<_> = default_accounts()
            .into_iter()
            .filter(|account| account.unit == UnitId::SEED_AUD)
            .collect();
        let holdings = adapter.holdings(UnitId::SEED_AUD);
        assert_eq!(
            holdings.iter().map(|h| h.name.clone()).collect::<Vec<_>>(),
            aud.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
        );
        let held = holdings
            .iter()
            .find(|holding| holding.name == "ANZ Everyday")
            .expect("ANZ Everyday counts in aud");
        assert_eq!(held.transactions, 2);
        assert_eq!(
            adapter.unit_usage(UnitId::SEED_AUD),
            UnitUse {
                accounts: aud.len(),
                ..UnitUse::default()
            }
        );
        assert_eq!(adapter.unit_usage(UnitId::new()), UnitUse::default());
        let bills = adapter.bill_accounts();
        assert_eq!(bills.len(), default_accounts().len());
        assert_eq!(bills[1].id, everyday);
        assert!(bills[1].takes_transactions);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_database_loads_no_accounts(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let (writer, _failures) = Writer::spawn(options).expect("the test database opens");
        let adapter = AccountsAdapter::load(PendingWrites::new(writer)).expect("it loads");
        assert!(adapter.accounts().accounts().is_empty());
    }
}
