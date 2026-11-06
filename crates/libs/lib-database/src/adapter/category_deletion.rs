//! The Category Deletion adapter: `lib-category-deletion`'s atomic port over the Categories,
//! Transactions, Bills, Payees and Budgets stores (ADR-0046), after the Tag Removal adapter. It
//! holds no rows of its own. A delete's writes go to each store in the foreign keys' order: the
//! Uncategorised home it made, the Transactions, the Bill Plans, the Payees and the Budgets (each
//! whole), then the Category's delete. Every write joins the [`PendingWrites`] every adapter
//! shares, so the bin's one commit after the call sends them as one transaction: a refused
//! Category delete takes the re-pointed rows down with it. `splits`, `bill_plans`, `payees` and
//! `budget_limit_records` name a Category with no cascade, so none may name it when it goes.
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_bills::BillPlan;
use lib_budgets::Budget;
use lib_categories::Category;
use lib_category_deletion::protocols::budgets_limiting;
use lib_category_deletion::{CategoryDeletion, CategoryRemoval};
use lib_core::CategoryId;
use lib_payees::Payee;
use lib_transactions::Transaction;

use super::category_rows::StoredCategoryRows;
use super::store::{
    SharedBillsStore, SharedBudgetsStore, SharedCategoriesStore, SharedPayeesStore,
    SharedTransactionsStore,
};

/// The deletion port over the stores the Categories, Transactions, Bills, Payees and Budgets
/// modules also hold. The rows naming the Category are read through [`StoredCategoryRows`], the
/// one selection the Categories handover and its counts read too (#1034).
pub struct CategoryDeletionAdapter {
    categories: SharedCategoriesStore,
    rows: StoredCategoryRows,
    budgets: SharedBudgetsStore,
}

impl CategoryDeletionAdapter {
    pub fn new(
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
        bills: SharedBillsStore,
        payees: SharedPayeesStore,
        budgets: SharedBudgetsStore,
    ) -> Self {
        Self {
            categories,
            rows: StoredCategoryRows {
                transactions,
                bills,
                payees,
            },
            budgets,
        }
    }
}

impl CategoryDeletion for CategoryDeletionAdapter {
    fn categories(&self) -> Vec<Category> {
        self.categories.borrow().categories().categories()
    }

    fn transactions_using(&self, id: CategoryId) -> Vec<Transaction> {
        self.rows.transactions_using(id)
    }

    fn bill_plans_using(&self, id: CategoryId) -> Vec<BillPlan> {
        self.rows.bill_plans_using(id)
    }

    fn payees_defaulting_to(&self, id: CategoryId) -> Vec<Payee> {
        self.rows.payees_defaulting_to(id)
    }

    fn budgets_limiting(&self, id: CategoryId) -> Vec<Budget> {
        budgets_limiting(self.budgets.borrow().budgets().budgets(), id)
    }

    /// The home first, since the moved rows name it, and the Category last, once nothing names it.
    fn remove(&mut self, removal: CategoryRemoval) {
        let CategoryRemoval {
            home,
            transactions,
            bill_plans,
            payees,
            budgets,
            deleted,
        } = removal;
        if let Some(home) = home {
            self.categories
                .borrow_mut()
                .categories_mut()
                .upsert_category(home);
        }
        {
            let mut store = self.rows.transactions.borrow_mut();
            for transaction in transactions {
                store.upsert_transaction(transaction);
            }
        }
        {
            let mut store = self.rows.bills.borrow_mut();
            for plan in bill_plans {
                store.plans_mut().upsert_bill_plan(plan);
            }
        }
        {
            let mut store = self.rows.payees.borrow_mut();
            for payee in payees {
                store.payees_mut().upsert_payee(payee);
            }
        }
        {
            let mut store = self.budgets.borrow_mut();
            for budget in &budgets {
                store.budgets_mut().upsert_budget(budget);
            }
        }
        self.categories.borrow_mut().delete_category(deleted);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_bills::BillPlanRepository as _;
    use lib_budgets::BudgetRepository as _;
    use lib_categories::CategoryRepository as _;
    use lib_category_deletion::contract::{self, DeletionFactory, Stored};
    use lib_category_deletion::fake::InMemoryCategoryDeletion;
    use lib_category_deletion::fake::world::world;
    use lib_transactions::TransactionRepository as _;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{
        AccountsAdapter, BillsAdapter, BudgetsAdapter, CategoriesAdapter, PayeesAdapter,
        PendingWrites, StoredCategoryDeletionReads, TransactionsAdapter, Writer,
    };

    /// The five stores, shared, as the adapter and the reads take them.
    struct SharedStores {
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
        bills: SharedBillsStore,
        payees: SharedPayeesStore,
        budgets: SharedBudgetsStore,
    }

    /// The five adapters on one test database and one pending batch, as start-up opens them. The
    /// sample's Units, Institutions and Accounts are stored first, which the world's rows name.
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
                lib_units::fake::InMemoryPorts::sample(chrono::Utc::now()),
            )
            .expect("the sample Units are written");
            crate::InstitutionsAdapter::seed(&writer, crate::seed::sample_institutions())
                .expect("the sample Institutions are written");
            CategoriesAdapter::seed(&writer, crate::seed::sample_categories())
                .expect("the sample Categories are written");
            AccountsAdapter::seed(&writer, crate::seed::sample_accounts())
                .expect("the sample Accounts are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// The five domains' rows stored now, read with fresh adapters as a restart would.
        fn read(&self) -> Stored {
            self.writer.flush().expect("the writer is running");
            let fresh = PendingWrites::new(self.writer.clone());
            let categories = CategoriesAdapter::load(fresh.clone()).expect("the Categories load");
            let transactions =
                TransactionsAdapter::load(fresh.clone()).expect("the Transactions load");
            let bills = BillsAdapter::load(fresh.clone()).expect("the Bills load");
            let payees = PayeesAdapter::load(fresh.clone()).expect("the Payees load");
            let budgets = BudgetsAdapter::load(fresh).expect("the Budgets load");
            Stored::new(
                categories.categories(),
                transactions.transactions(),
                bills.bill_plans(),
                payees.payees().payees(),
                budgets.budgets(),
            )
        }

        /// Commits, settles the writes, and reads back what is stored, with the failures reported.
        fn settle(&mut self) -> (Stored, usize) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            (self.read(), failures)
        }

        /// The world's rows stored, and the five stores loaded on the one pending batch.
        fn stores(&self, rows: InMemoryCategoryDeletion) -> SharedStores {
            CategoriesAdapter::seed(&self.writer, rows.categories.categories)
                .expect("the Categories are written");
            PayeesAdapter::seed(&self.writer, rows.payees.payees).expect("the Payees are written");
            TransactionsAdapter::seed(&self.writer, rows.transactions.transactions)
                .expect("the Transactions are written");
            BillsAdapter::seed(&self.writer, rows.bills.plans, rows.bills.entries)
                .expect("the Bills are written");
            BudgetsAdapter::seed(&self.writer, rows.budgets.budgets)
                .expect("the Budgets are written");
            SharedStores {
                categories: Rc::new(RefCell::new(
                    CategoriesAdapter::load(self.pending.clone()).expect("the Categories load"),
                )),
                transactions: Rc::new(RefCell::new(
                    TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load"),
                )),
                bills: Rc::new(RefCell::new(
                    BillsAdapter::load(self.pending.clone()).expect("the Bills load"),
                )),
                payees: Rc::new(RefCell::new(
                    PayeesAdapter::load(self.pending.clone()).expect("the Payees load"),
                )),
                budgets: Rc::new(RefCell::new(
                    BudgetsAdapter::load(self.pending.clone()).expect("the Budgets load"),
                )),
            }
        }
    }

    impl DeletionFactory for AdapterFactory {
        type Ports = CategoryDeletionAdapter;

        fn create(&mut self, rows: InMemoryCategoryDeletion) -> CategoryDeletionAdapter {
            let SharedStores {
                categories,
                transactions,
                bills,
                payees,
                budgets,
            } = self.stores(rows);
            CategoryDeletionAdapter::new(categories, transactions, bills, payees, budgets)
        }

        fn uncommitted(&mut self) -> Stored {
            self.read()
        }

        fn stored(&mut self, _ports: CategoryDeletionAdapter) -> Stored {
            let (stored, failures) = self.settle();
            assert_eq!(failures, 0, "a write failed");
            stored
        }

        fn break_category_deletes(&mut self, _ports: &mut CategoryDeletionAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_category_delete BEFORE DELETE ON categories \
                         BEGIN SELECT RAISE(ABORT, 'deletes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, _ports: CategoryDeletionAdapter) -> (Stored, usize) {
            self.settle()
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_moves_its_rows_to_a_new_uncategorised(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_delete_moves_its_rows_to_a_new_uncategorised(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_delete_reuses_the_uncategorised_of_its_type(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_delete_reuses_the_uncategorised_of_its_type(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_unused_delete_makes_no_uncategorised(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::an_unused_delete_makes_no_uncategorised(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_writes_wait_for_the_one_commit(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_writes_wait_for_the_one_commit(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failing_delete_lands_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_failing_delete_lands_nothing(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refusal_writes_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_refusal_writes_nothing(&mut AdapterFactory::new(options));
    }

    /// Over the adapters, the Dialog's reads count the Splits and Budget Limits in the file.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_stored_reads_quote_a_category_in_use(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        let SharedStores {
            categories,
            transactions,
            budgets,
            ..
        } = factory.stores(world());

        contract::the_reads_quote_a_category_in_use(&StoredCategoryDeletionReads {
            categories,
            transactions,
            budgets,
        });
    }
}
