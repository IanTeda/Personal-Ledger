//! The Account Deletion adapter: `lib-account-deletion`'s atomic port over the Accounts,
//! Transactions and Bills stores (ADR-0046), after the Bill Payments adapter. It holds no rows of
//! its own. A delete's Unmatched entries go to the Bills store, its Transactions' removal to the
//! Transactions store and the Account's to the Accounts store, in that order, and every write joins
//! the [`PendingWrites`] every adapter shares, so the bin's one commit after the call sends them as
//! one transaction: a refused Account delete takes the Transactions and the Unmatch down with it.
//! The file's foreign keys take the Account's Balance Checks with it.
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_account_deletion::{AccountDeletion, AccountRemoval};
use lib_accounts::Account;
use lib_bills::BillScheduleEntry;
use lib_core::{AccountId, TransactionId};

use super::store::{SharedAccountsStore, SharedBillsStore, SharedTransactionsStore};

/// The deletion port over the stores the Accounts, Transactions and Bills modules also hold.
pub struct AccountDeletionAdapter {
    accounts: SharedAccountsStore,
    transactions: SharedTransactionsStore,
    bills: SharedBillsStore,
}

impl AccountDeletionAdapter {
    pub fn new(
        accounts: SharedAccountsStore,
        transactions: SharedTransactionsStore,
        bills: SharedBillsStore,
    ) -> Self {
        Self {
            accounts,
            transactions,
            bills,
        }
    }
}

impl AccountDeletion for AccountDeletionAdapter {
    fn account(&self, id: AccountId) -> Option<Account> {
        self.accounts.borrow().accounts().account(id)
    }

    fn bill_plans_paying_from(&self, id: AccountId) -> usize {
        self.bills
            .borrow()
            .plans()
            .bill_plans()
            .iter()
            .filter(|plan| plan.account_id == id)
            .count()
    }

    fn transactions_booked_to(&self, id: AccountId) -> Vec<TransactionId> {
        self.transactions
            .borrow()
            .all_transactions()
            .iter()
            .filter(|transaction| transaction.account_id == id)
            .map(|transaction| transaction.id)
            .collect()
    }

    fn bill_entries(&self) -> Vec<BillScheduleEntry> {
        self.bills.borrow().entries().entries()
    }

    /// The entries first, then the Transactions, then the Account: each row goes before what it
    /// names does, a Paid link before its Split and a Transaction before its Account.
    fn remove_account(&mut self, removal: AccountRemoval) {
        let AccountRemoval {
            account,
            transactions,
            unmatched,
        } = removal;
        {
            let mut bills = self.bills.borrow_mut();
            for entry in unmatched {
                bills.entries_mut().upsert_entry(entry);
            }
        }
        {
            let mut store = self.transactions.borrow_mut();
            for id in transactions {
                store.transactions_mut().delete_transaction(id);
            }
        }
        self.accounts
            .borrow_mut()
            .accounts_mut()
            .delete_account(account);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_account_deletion::contract::{self, DeletionFactory, Stored};
    use lib_account_deletion::fake::InMemoryDeletion;
    use lib_account_deletion::fake::world::{WALLET, world};
    use lib_account_deletion::interactor::DeleteAccount;
    use lib_bills::{BillPlanRepository as _, BillScheduleRepository as _};
    use lib_transactions::TransactionRepository as _;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{AccountsAdapter, BillsAdapter, PendingWrites, TransactionsAdapter, Writer};

    /// The three adapters on one test database and one pending batch, as start-up opens them. The
    /// stub's Units and Categories are stored first, which the Accounts, Splits and the Bill Plan
    /// name.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::UnitsAdapter::seed_from(
                &writer,
                lib_units::fake::InMemoryPorts::stub(chrono::Utc::now()),
            )
            .expect("the stub Units are written");
            crate::InstitutionsAdapter::seed(&writer, crate::seed::stub_institutions())
                .expect("the stub Institutions are written");
            crate::CategoriesAdapter::seed(&writer, crate::seed::stub_categories())
                .expect("the stub Categories are written");
            crate::TagsAdapter::seed(&writer, crate::seed::stub_tags())
                .expect("the stub Tags are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// Commits, settles the writes, and reads back what is stored with fresh adapters, as a
        /// restart would.
        fn settle(&mut self) -> (Stored, usize) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            let fresh = PendingWrites::new(self.writer.clone());
            let accounts = AccountsAdapter::load(fresh.clone()).expect("the Accounts load");
            let transactions =
                TransactionsAdapter::load(fresh.clone()).expect("the Transactions load");
            let bills = BillsAdapter::load(fresh).expect("the Bills load");
            let stored = Stored::new(
                accounts.accounts().accounts(),
                transactions.transactions(),
                bills.bill_plans(),
                bills.entries(),
            );
            (stored, failures)
        }

        /// How many Balance Checks are stored for `account`.
        fn balance_checks(&self, account: AccountId) -> i64 {
            self.writer
                .run(move |pool| async move {
                    let count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM balance_checks WHERE account_id = ?",
                    )
                    .bind(account)
                    .fetch_one(&pool)
                    .await?;
                    Ok(count)
                })
                .expect("the Balance Checks are counted")
        }
    }

    impl DeletionFactory for AdapterFactory {
        type Ports = AccountDeletionAdapter;

        fn create(&mut self, rows: InMemoryDeletion) -> AccountDeletionAdapter {
            AccountsAdapter::seed(&self.writer, rows.accounts.accounts)
                .expect("the Accounts are written");
            TransactionsAdapter::seed(&self.writer, rows.transactions.transactions)
                .expect("the Transactions are written");
            BillsAdapter::seed(&self.writer, rows.bills.plans, rows.bills.entries)
                .expect("the Bills are written");
            let accounts = AccountsAdapter::load(self.pending.clone()).expect("the Accounts load");
            let transactions =
                TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load");
            let bills = BillsAdapter::load(self.pending.clone()).expect("the Bills load");
            AccountDeletionAdapter::new(
                Rc::new(RefCell::new(accounts)),
                Rc::new(RefCell::new(transactions)),
                Rc::new(RefCell::new(bills)),
            )
        }

        fn stored(&mut self, _ports: AccountDeletionAdapter) -> Stored {
            let (stored, failures) = self.settle();
            assert_eq!(failures, 0, "a write failed");
            stored
        }

        fn break_account_deletes(&mut self, _ports: &mut AccountDeletionAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_account_delete BEFORE DELETE ON accounts \
                         BEGIN SELECT RAISE(ABORT, 'deletes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, _ports: AccountDeletionAdapter) -> (Stored, usize) {
            self.settle()
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_lands_the_account_its_transactions_and_their_unmatch(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_delete_lands_the_account_its_transactions_and_their_unmatch(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failing_delete_lands_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_failing_delete_lands_nothing(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_account_a_bill_plan_pays_from_is_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_account_a_bill_plan_pays_from_is_refused(&mut AdapterFactory::new(options));
    }

    /// The port doesn't hold Balance Checks: the file's cascade takes them with the Account.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_accounts_balance_checks_go_with_it(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut ports = factory.create(world());
        factory
            .writer
            .run(|pool| async move {
                sqlx::query(
                    "INSERT INTO balance_checks (id, account_id, date, asserted_balance)
                     VALUES (?, ?, '2026-09-01', '0')",
                )
                .bind(lib_core::RowID::new())
                .bind(WALLET)
                .execute(&pool)
                .await?;
                Ok(())
            })
            .expect("the Balance Check is written");
        assert_eq!(factory.balance_checks(WALLET), 1);

        DeleteAccount { id: WALLET }
            .run(&mut ports)
            .expect("nothing pays from the Wallet");
        factory.stored(ports);

        assert_eq!(factory.balance_checks(WALLET), 0);
    }
}
