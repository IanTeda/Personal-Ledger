//! The in-memory deletion port, for tests and UI-first work, which answers the Dialog's reads from
//! its own rows too, and [`world`], the small Ledger the tests and the contract delete from. It
//! holds its rows in plain fields so a test can set up or read state directly. The Dialog's Router
//! is `lib-categories`' own recording one.

use std::cell::RefCell;
use std::rc::Rc;

use lib_bills::fake::InMemoryBills;
use lib_bills::{BillPlan, BillPlanRepository};
use lib_budgets::fake::InMemoryBudgets;
use lib_budgets::{Budget, BudgetRepository};
use lib_categories::fake::InMemoryCategories;
use lib_categories::{Category, CategoryRepository};
use lib_core::CategoryId;
use lib_payees::fake::InMemoryPayees;
use lib_payees::{Payee, PayeeRepository};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};

use crate::protocols::{
    CategoryDeletion, CategoryDeletionReads, CategoryInUse, CategoryRemoval,
    SharedCategoryDeletion, bill_plans_using, budgets_limiting, payees_defaulting_to,
    transactions_using,
};

/// A delete's five stores in memory, written together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryCategoryDeletion {
    pub categories: InMemoryCategories,
    pub transactions: InMemoryTransactions,
    pub bills: InMemoryBills,
    pub payees: InMemoryPayees,
    pub budgets: InMemoryBudgets,
}

impl InMemoryCategoryDeletion {
    /// This port shared, as the Dialog holds it, with a handle a test reads it back through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedCategoryDeletion) {
        let rows = Rc::new(RefCell::new(self));
        let shared: SharedCategoryDeletion = rows.clone();
        (rows, shared)
    }
}

impl CategoryDeletion for InMemoryCategoryDeletion {
    fn categories(&self) -> Vec<Category> {
        self.categories.categories()
    }

    fn transactions_using(&self, id: CategoryId) -> Vec<Transaction> {
        transactions_using(self.transactions.transactions(), id)
    }

    fn bill_plans_using(&self, id: CategoryId) -> Vec<BillPlan> {
        bill_plans_using(self.bills.bill_plans(), id)
    }

    fn payees_defaulting_to(&self, id: CategoryId) -> Vec<Payee> {
        payees_defaulting_to(self.payees.payees(), id)
    }

    fn budgets_limiting(&self, id: CategoryId) -> Vec<Budget> {
        budgets_limiting(self.budgets.budgets(), id)
    }

    fn remove(&mut self, removal: CategoryRemoval) {
        if let Some(home) = removal.home {
            self.categories.upsert_category(home);
        }
        for transaction in removal.transactions {
            self.transactions.upsert_transaction(transaction);
        }
        for plan in removal.bill_plans {
            self.bills.upsert_bill_plan(plan);
        }
        for payee in removal.payees {
            self.payees.upsert_payee(payee);
        }
        for budget in &removal.budgets {
            self.budgets.upsert_budget(budget);
        }
        self.categories
            .categories
            .retain(|category| category.id != removal.deleted);
    }
}

/// Counted from the rows held, so a test that books a Split sees it in the Dialog's figures.
impl CategoryDeletionReads for InMemoryCategoryDeletion {
    fn category_in_use(&self, id: CategoryId) -> Option<CategoryInUse> {
        Some(CategoryInUse {
            category: self.categories.category(id)?,
            splits: self
                .transactions
                .transactions
                .iter()
                .flat_map(|transaction| &transaction.splits)
                .filter(|split| split.category_id == id)
                .count(),
            budgets: budgets_limiting(self.budgets.budgets(), id).len(),
        })
    }
}

/// The small Ledger the tests and the contract delete from, as of 11 September 2026: `lib-
/// categories`' sample Categories and an Income Uncategorised; five Transactions on the sample's ANZ
/// Everyday, three with Splits on Groceries, Dining or Rent, one on Salary and one on the Income
/// Uncategorised; two Bill Plans, on Groceries and Rent; `lib-payees`' sample Payees, three of which
/// default to Groceries; and `lib-budgets`' sample Budgets, two of which hold Groceries Limits. No Expense
/// Uncategorised is held, so moving an Expense Category's rows makes one.
pub mod world {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_bills::fake::InMemoryBills;
    use lib_bills::{AmountKind, BillPlan, Recurrence};
    use lib_budgets::fake::InMemoryBudgets;
    use lib_categories::Category;
    use lib_categories::fake::InMemoryCategories;
    use lib_core::{
        AccountId, BillPlanId, CategoryId, CategoryTypes, Money, TransactionId, TransactionStatus,
        UnitId,
    };
    use lib_payees::fake::InMemoryPayees;
    use lib_transactions::fake::InMemoryTransactions;
    use lib_transactions::{Split, Transaction};

    use super::InMemoryCategoryDeletion;

    /// A parent, of Groceries and Dining.
    pub const FOOD: CategoryId = CategoryId::seed(6);
    /// Two Splits, a Bill Plan, three Payee defaults and two Budgets' Limits name it.
    pub const GROCERIES: CategoryId = CategoryId::seed(7);
    /// One Split names it.
    pub const DINING: CategoryId = CategoryId::seed(8);
    /// One Split and a Bill Plan name it.
    pub const RENT: CategoryId = CategoryId::seed(2);
    /// No Split or Bill Plan names it.
    pub const WATER: CategoryId = CategoryId::seed(5);
    /// One Split names it.
    pub const SALARY: CategoryId = CategoryId::seed(11);
    /// The Income Uncategorised, which one Split names.
    pub const UNCATEGORISED_INCOME: CategoryId = CategoryId::seed(13);

    /// Its one Split is on Groceries.
    pub const GROCERIES_ONLY: TransactionId = TransactionId::seed(1);
    /// Its first Split is on Groceries, its second on Dining.
    pub const GROCERIES_AND_DINING: TransactionId = TransactionId::seed(2);
    pub const RENT_PAID: TransactionId = TransactionId::seed(3);
    pub const PAY_DAY: TransactionId = TransactionId::seed(4);
    /// Its one Split is on the Income Uncategorised.
    pub const UNSORTED_INTEREST: TransactionId = TransactionId::seed(5);

    /// On Groceries.
    pub const GROCERY_BOX: BillPlanId = BillPlanId::seed(1);
    /// On Rent.
    pub const LEASE: BillPlanId = BillPlanId::seed(2);

    /// The sample's ANZ Everyday, which the file's foreign keys need.
    const EVERYDAY: AccountId = AccountId::seed(2);

    /// The day the sample Budgets are seeded as of.
    pub fn today() -> NaiveDate {
        date(11)
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap_or_default()
    }

    fn cents(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    fn split(cents_: i64, category_id: CategoryId) -> Split {
        Split {
            amount: cents(cents_),
            category_id,
            payee_id: None,
            tag_ids: Vec::new(),
        }
    }

    fn transaction(id: TransactionId, day: u32, splits: Vec<Split>) -> Transaction {
        Transaction {
            id,
            date: date(day),
            account_id: EVERYDAY,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some("Groceries".to_string()),
            splits,
        }
    }

    fn plan(id: BillPlanId, name: &str, category_id: CategoryId) -> BillPlan {
        BillPlan {
            id,
            name: name.to_string(),
            category_id,
            unit: UnitId::SEED_AUD,
            account_id: EVERYDAY,
            payee_id: None,
            planned_amount: cents(8_999),
            amount_kind: AmountKind::Fixed,
            recurrence: Recurrence::Monthly,
            first_due: date(1),
            ends_on: None,
            attention_lead: Some(5),
            is_active: true,
            generate_from: date(1),
        }
    }

    /// The rows described above.
    pub fn world() -> InMemoryCategoryDeletion {
        let mut categories = InMemoryCategories::sample();
        categories.categories.push(Category {
            id: UNCATEGORISED_INCOME,
            name: lib_categories::UNCATEGORISED_NAME.to_string(),
            parent: None,
            category_type: CategoryTypes::Income,
        });
        InMemoryCategoryDeletion {
            categories,
            transactions: InMemoryTransactions {
                transactions: vec![
                    transaction(GROCERIES_ONLY, 1, vec![split(-4_250, GROCERIES)]),
                    transaction(
                        GROCERIES_AND_DINING,
                        2,
                        vec![split(-1_999, GROCERIES), split(-3_000, DINING)],
                    ),
                    transaction(RENT_PAID, 3, vec![split(-52_000, RENT)]),
                    transaction(PAY_DAY, 4, vec![split(350_000, SALARY)]),
                    transaction(
                        UNSORTED_INTEREST,
                        5,
                        vec![split(1_250, UNCATEGORISED_INCOME)],
                    ),
                ],
            },
            bills: InMemoryBills {
                plans: vec![
                    plan(GROCERY_BOX, "Grocery box", GROCERIES),
                    plan(LEASE, "Lease", RENT),
                ],
                entries: Vec::new(),
            },
            payees: InMemoryPayees {
                payees: lib_payees::sample_payees(),
            },
            budgets: InMemoryBudgets::sample(today()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::world::{FOOD, GROCERIES, RENT, WATER, world};
    use super::*;

    #[test]
    fn the_world_holds_what_its_doc_says() {
        let world = world();

        assert_eq!(world.transactions_using(GROCERIES).len(), 2);
        assert_eq!(world.bill_plans_using(GROCERIES).len(), 1);
        assert_eq!(world.payees_defaulting_to(GROCERIES).len(), 3);
        assert_eq!(world.budgets_limiting(GROCERIES).len(), 2);
        assert!(!world.budgets_limiting(GROCERIES).is_empty());
        assert!(world.transactions_using(WATER).is_empty());
        assert!(world.bill_plans_using(WATER).is_empty());
        assert!(
            !world
                .categories()
                .iter()
                .any(|category| lib_categories::is_uncategorised(category)
                    && category.category_type == lib_core::CategoryTypes::Expense),
            "no Expense Uncategorised yet"
        );
    }

    #[test]
    fn the_reads_count_the_held_splits_and_budgets() {
        let world = world();

        let rent = world.category_in_use(RENT).expect("Rent is held");
        assert_eq!((rent.category.name.as_str(), rent.splits), ("Rent", 1));
        let groceries = world.category_in_use(GROCERIES).expect("Groceries is held");
        assert_eq!(groceries.splits, 2);
        assert_eq!(groceries.budgets, world.budgets_limiting(GROCERIES).len());
        assert_eq!(world.category_in_use(FOOD).map(|food| food.splits), Some(0));
        assert_eq!(world.category_in_use(CategoryId::seed(999)), None);
    }
}
