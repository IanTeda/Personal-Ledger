//! The Categories adapter: `lib-categories`' repository port over the `categories` table, and the
//! reads other modules make of the same rows: `lib-bills`' `BillCategories`, `lib-payees`'
//! `PayeeCategories` and `lib-transactions`' `TransactionCategories`, each flag and path filled by
//! `lib-categories`' own rule (ADR-0046, #858, #925).
//!
//! It loads every Category into memory, blocking, and serves the rows from there in tree order,
//! siblings in the order they were added (rowid order, which an upsert keeps). A port write
//! changes memory at once and joins the [`PendingWrites`] every adapter shares, which the bin
//! commits after each input.
//!
//! The port has no delete: Category delete re-points other domains' rows, so it stays a
//! `lib-ledger` command until `lib-reassign` (#813). [`CategoriesAdapter::delete_category`] is the
//! write the Desktop's forwarding shim sends that command's delete through, after the Splits, Bill
//! Plans and Payee defaults it re-points: their foreign keys, and `parent_id`'s, refuse deleting a
//! Category anything still names.

use lib_bills::{BillCategories, BillCategory};
use lib_categories::{Category, CategoryRepository, tree_order};
use lib_core::{CategoryId, CategoryTypes};
use lib_payees::{PayeeCategories, PayeeCategory};
use lib_transactions::{TransactionCategories, TransactionCategory};
use sqlx::SqliteConnection;

use crate::Result;
use crate::commit::PendingWrites;
use crate::writer::{BoxFuture, Change, Domain, Writer};

/// One write, saved in the order it was made.
#[derive(Debug, Clone)]
enum CategoriesChange {
    Upsert(Category),
    Delete(CategoryId),
}

/// The Categories domain, held in memory and saved through the shared writer.
pub struct CategoriesAdapter {
    /// In the order added; [`CategoryRepository::categories`] lists them in tree order.
    rows: Vec<Category>,
    pending: PendingWrites,
}

impl CategoriesAdapter {
    /// Loads every Category through `pending`'s writer, blocking until they are read. The
    /// adapter's writes join `pending`. An empty database loads no rows.
    ///
    /// # Errors
    /// Returns the error of the read.
    #[tracing::instrument(name = "Load the Categories", level = "debug", skip_all, err)]
    pub fn load(pending: PendingWrites) -> Result<Self> {
        let rows = pending.writer().run(|pool| async move {
            let mut conn = pool.acquire().await?;
            read(&mut conn).await
        })?;
        Ok(Self { rows, pending })
    }

    /// Writes `categories` in one transaction, blocking until it lands, in tree order so each
    /// parent is stored before its children. Each is an upsert, so seeding again leaves the same
    /// rows. For before the Payees, Splits and Bill Plans that name them.
    ///
    /// # Errors
    /// Returns the error of the write; nothing is written then.
    #[tracing::instrument(name = "Seed the Categories", level = "debug", skip_all, err)]
    pub fn seed(writer: &Writer, categories: Vec<Category>) -> Result<()> {
        writer.run(move |pool| async move {
            let mut conn = pool.begin().await?;
            Self::seed_in(&mut conn, &categories).await?;
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
        categories: &[Category],
    ) -> Result<()> {
        for category in &tree_order(categories) {
            upsert(conn, category).await?;
        }
        Ok(())
    }

    /// Drops the Categories changes not yet committed and loads the rows again, blocking. For
    /// after a [`WriteFailure`](crate::writer::WriteFailure) names [`Domain::Categories`], so
    /// memory matches the file again.
    ///
    /// # Errors
    /// Returns the error of the read; memory is left as it was then.
    #[tracing::instrument(name = "Reload the Categories", level = "debug", skip_all, err)]
    pub fn reload(&mut self) -> Result<()> {
        let fresh = Self::load(self.pending.clone())?;
        self.rows = fresh.rows;
        self.pending.discard(Domain::Categories);
        Ok(())
    }

    /// Removes Category `id`: Category reassign's delete, which the Desktop's forwarding shim
    /// sends once the rows naming it are re-pointed (#813 moves it to `lib-reassign`).
    pub fn delete_category(&mut self, id: CategoryId) {
        self.rows.retain(|row| row.id != id);
        self.pending.push(CategoriesChange::Delete(id));
    }
}

impl CategoryRepository for CategoriesAdapter {
    fn categories(&self) -> Vec<Category> {
        tree_order(&self.rows)
    }

    fn upsert_category(&mut self, category: Category) {
        match self.rows.iter_mut().find(|row| row.id == category.id) {
            Some(row) => *row = category.clone(),
            None => self.rows.push(category.clone()),
        }
        self.pending.push(CategoriesChange::Upsert(category));
    }
}

impl BillCategories for CategoriesAdapter {
    fn bill_categories(&self) -> Vec<BillCategory> {
        bill_categories(&self.categories())
    }
}

impl PayeeCategories for CategoriesAdapter {
    fn payee_categories(&self) -> Vec<PayeeCategory> {
        payee_categories(&self.categories())
    }
}

impl TransactionCategories for CategoriesAdapter {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        let categories = self.categories();
        categories
            .iter()
            .map(|category| TransactionCategory {
                id: category.id,
                name: category.name.clone(),
                parent: category.parent,
                path: lib_categories::path(&categories, category.id)
                    .unwrap_or_else(|| category.name.clone()),
            })
            .collect()
    }
}

/// `categories` as Bills reads them, in order: each with its path, and whether it is an Expense
/// and a leaf by `lib-categories`' rules.
pub(crate) fn bill_categories(categories: &[Category]) -> Vec<BillCategory> {
    categories
        .iter()
        .map(|category| BillCategory {
            id: category.id,
            name: category.name.clone(),
            path: lib_categories::path(categories, category.id)
                .unwrap_or_else(|| category.name.clone()),
            is_expense: category.category_type == CategoryTypes::Expense,
            is_leaf: lib_categories::is_leaf(categories, category.id),
        })
        .collect()
}

/// `categories`' leaves as a Payee's Default category reads them, in order. A leaf whose name
/// another leaf shares is labelled under its parent (`Parent › Leaf`), so a label always names
/// exactly one Category.
pub(crate) fn payee_categories(categories: &[Category]) -> Vec<PayeeCategory> {
    let leaves: Vec<&Category> = categories
        .iter()
        .filter(|category| lib_categories::is_leaf(categories, category.id))
        .collect();
    leaves
        .iter()
        .map(|leaf| {
            let shared = leaves
                .iter()
                .filter(|other| other.name == leaf.name)
                .count()
                > 1;
            let parent = leaf
                .parent
                .and_then(|id| categories.iter().find(|c| c.id == id));
            PayeeCategory {
                id: leaf.id,
                label: match parent {
                    Some(parent) if shared => {
                        format!(
                            "{}{}{}",
                            parent.name,
                            lib_categories::PATH_SEPARATOR,
                            leaf.name
                        )
                    }
                    _ => leaf.name.clone(),
                },
            }
        })
        .collect()
}

impl Change for CategoriesChange {
    fn domain(&self) -> Domain {
        Domain::Categories
    }

    fn apply<'c>(self: Box<Self>, conn: &'c mut SqliteConnection) -> BoxFuture<'c, Result<()>> {
        Box::pin(async move {
            match *self {
                Self::Upsert(category) => upsert(conn, &category).await,
                Self::Delete(id) => delete(conn, id).await,
            }
        })
    }
}

async fn upsert(conn: &mut SqliteConnection, category: &Category) -> Result<()> {
    sqlx::query!(
        "INSERT INTO categories (id, name, parent_id, category_type)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name,
             parent_id = excluded.parent_id,
             category_type = excluded.category_type",
        category.id,
        category.name,
        category.parent,
        category.category_type,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn delete(conn: &mut SqliteConnection, id: CategoryId) -> Result<()> {
    sqlx::query!("DELETE FROM categories WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn read(conn: &mut SqliteConnection) -> Result<Vec<Category>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: CategoryId",
                  name,
                  parent_id AS "parent_id: CategoryId",
                  category_type AS "category_type!: CategoryTypes"
           FROM categories
           ORDER BY rowid"#
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| Category {
        id: row.id,
        name: row.name,
        parent: row.parent_id,
        category_type: row.category_type,
    })
    .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_categories::contract::{self, PortsFactory, StoredPorts};
    use lib_categories::default_categories;
    use lib_core::{AccountId, Money, PayeeId, TransactionId, TransactionStatus};
    use lib_transactions::{Split, Transaction};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::test_support::TransactionReadsFactory;
    use crate::writer::WriteFailure;
    use crate::{PayeesAdapter, TransactionsAdapter};

    impl StoredPorts for CategoriesAdapter {
        fn categories(&mut self) -> &mut dyn CategoryRepository {
            self
        }
    }

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
        type Ports = CategoriesAdapter;

        fn create(&mut self, seed: Vec<Category>) -> CategoriesAdapter {
            CategoriesAdapter::seed(&self.writer, seed).expect("the seed is written");
            CategoriesAdapter::load(self.pending.clone()).expect("the seed loads")
        }

        fn reload(&mut self, _ports: CategoriesAdapter) -> CategoriesAdapter {
            self.pending.commit();
            let fresh = CategoriesAdapter::load(self.pending.clone()).expect("the rows load");
            if let Some(failure) = self.failures().first() {
                panic!("a write failed: {failure:?}");
            }
            fresh
        }

        fn commit(&mut self, _ports: &mut CategoriesAdapter) {
            self.pending.commit();
        }

        fn break_writes(&mut self, _ports: &mut CategoriesAdapter) {
            self.writer
                .run(|pool| async move {
                    sqlx::query(
                        "CREATE TRIGGER refuse_categories BEFORE INSERT ON categories
                         BEGIN SELECT RAISE(ABORT, 'writes are refused'); END",
                    )
                    .execute(&pool)
                    .await?;
                    Ok(())
                })
                .expect("the trigger is created");
        }

        fn recover(&mut self, mut ports: CategoriesAdapter) -> (CategoriesAdapter, usize) {
            let failures = self
                .failures()
                .iter()
                .filter(|failure| failure.domains == [Domain::Categories])
                .count();
            ports.reload().expect("the rows load");
            (ports, failures)
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn added_categories_reload_the_same(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::added_categories_reload_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_renamed_category_reloads_the_same(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_renamed_category_reloads_the_same(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_moved_category_reloads_under_its_new_parent(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_moved_category_reloads_under_its_new_parent(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn categories_list_in_tree_order(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::categories_list_in_tree_order(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refused_write_leaves_the_tree_as_it_was(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_refused_write_leaves_the_tree_as_it_was(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failed_write_reloads_and_reports(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failed_write_reloads_and_reports(&mut AdapterFactory::new(options));
    }

    /// Category reassign's delete: once nothing names the Category it is gone after a reload.
    #[sqlx::test(migrations = "migrations/client")]
    async fn a_deleted_category_is_gone_after_a_reload(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut categories = factory.create(default_categories());
        let dining = CategoryId::seed(8);

        categories.delete_category(dining);
        let categories = factory.reload(categories);

        assert!(categories.category(dining).is_none());
        assert_eq!(
            categories.categories().len(),
            default_categories().len() - 1
        );
    }

    /// The Interactor's leaf rule is the person-facing refusal; `parent_id`'s key backs it, so a
    /// delete of a parent fails in the file and the tree stays whole.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_deleting_a_parent(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut categories = factory.create(default_categories());
        let stored = categories.categories();

        categories.delete_category(CategoryId::seed(6));
        factory.pending.commit();

        let failures = factory.failures();
        assert_eq!(failures.len(), 1, "Food still has Groceries and Dining");
        assert_eq!(failures[0].domains, [Domain::Categories]);
        categories.reload().expect("the rows load");
        assert_eq!(categories.categories(), stored);
    }

    /// `lib-transactions`' read check: each Category is stored as an Expense.
    #[sqlx::test(migrations = "migrations/client")]
    async fn transactions_read_the_categories_rows(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        fn store(writer: &Writer, rows: Vec<TransactionCategory>) {
            let categories = rows
                .into_iter()
                .map(|row| Category {
                    id: row.id,
                    name: row.name,
                    parent: row.parent,
                    category_type: CategoryTypes::Expense,
                })
                .collect();
            CategoriesAdapter::seed(writer, categories).expect("the Categories are written");
        }
        lib_transactions::contract::categories_read_back_in_tree_order(
            &mut TransactionReadsFactory::new(options, store, |pending| {
                CategoriesAdapter::load(pending).expect("the rows load")
            }),
        );
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// A Transaction with one Split filed under `category`.
    fn filed_under(category: CategoryId) -> Transaction {
        Transaction {
            id: TransactionId::new(),
            date: day(),
            account_id: AccountId::seed(1),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: vec![Split {
                amount: Money(BigDecimal::from(-10)),
                category_id: category,
                payee_id: None,
                tag_ids: Vec::new(),
            }],
        }
    }

    /// Category reassign re-points a Split before the delete; one that skips it fails in the file,
    /// as does a Split filed under a Category that was never stored.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_split_naming_no_stored_category(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        crate::test_support::store_stub_references(&factory.writer);
        let mut categories = CategoriesAdapter::load(factory.pending.clone()).expect("rows load");
        let groceries = CategoryId::seed(7);
        TransactionsAdapter::seed(&factory.writer, vec![filed_under(groceries)])
            .expect("Groceries is stored");

        categories.delete_category(groceries);
        factory.pending.commit();
        assert_eq!(factory.failures().len(), 1, "a Split is filed under it");

        assert!(
            TransactionsAdapter::seed(&factory.writer, vec![filed_under(CategoryId::new())])
                .is_err(),
            "no Category backs the Split"
        );
    }

    /// A Payee's Default category names a stored Category: Category reassign clears it before the
    /// delete, and a default naming no stored Category fails in the file.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_payee_default_naming_no_stored_category(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        crate::test_support::store_stub_references(&factory.writer);
        let payee = lib_payees::Payee {
            id: PayeeId::new(),
            name: "Corner Shop".into(),
            aliases: Vec::new(),
            default_category: Some(CategoryId::new()),
            is_active: true,
        };

        assert!(PayeesAdapter::seed(&factory.writer, vec![payee]).is_err());
    }

    /// A Bill Plan names a stored Category, as a Split does.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_file_refuses_a_bill_plan_naming_no_stored_category(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let factory = AdapterFactory::new(options);
        crate::test_support::store_stub_references(&factory.writer);
        let mut plan = crate::seed::stub_ledger_bills(day()).plans[0].clone();
        plan.id = lib_core::BillPlanId::new();
        plan.payee_id = None;
        plan.category_id = CategoryId::new();

        assert!(crate::BillsAdapter::seed(&factory.writer, vec![plan], Vec::new()).is_err());
    }

    /// The adapter answers Bills and Payees from its rows by `lib-categories`' rules, as the stub
    /// Ledger's own reads did: the fakes both modules test on are the stub's.
    #[sqlx::test(migrations = "migrations/client")]
    async fn the_reads_match_the_fakes_bills_and_payees_test_on(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let categories = factory.create(default_categories());

        assert_eq!(
            categories.bill_categories(),
            lib_bills::fake::stub_categories()
        );
        assert_eq!(
            categories.payee_categories(),
            lib_payees::fake::stub_categories()
        );
    }

    #[test]
    fn a_leaf_whose_name_another_leaf_shares_is_labelled_under_its_parent() {
        let mut categories = default_categories();
        categories.push(Category {
            id: CategoryId::new(),
            name: "Groceries".into(),
            parent: Some(CategoryId::seed(1)),
            category_type: CategoryTypes::Expense,
        });

        let labels: Vec<String> = payee_categories(&tree_order(&categories))
            .into_iter()
            .map(|category| category.label)
            .collect();

        assert!(labels.contains(&"Food \u{203a} Groceries".to_string()));
        assert!(labels.contains(&"Housing \u{203a} Groceries".to_string()));
        assert!(
            !labels.contains(&"Housing".to_string()),
            "a parent isn't offered"
        );
    }
}
