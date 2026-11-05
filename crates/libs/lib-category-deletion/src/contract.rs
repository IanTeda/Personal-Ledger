//! The contract every stored deletion port keeps (ADR-0046), run against both the in-memory port
//! here and the `lib-database` adapter, so the fake the Clients' tests use can be trusted: a delete
//! lands the moved Splits and Bill Plans, the cleared Payee defaults, the dropped Budget Limits and
//! the Category's delete together, in the one commit after the call and not before, in an order no
//! foreign key refuses; a write that fails lands none of it, and a refusal writes nothing. The
//! reads the Dialog opens on keep one check too.
//!
//! Each check is a function generic over a [`DeletionFactory`]: a consumer runs it from its own
//! test, since an adapter's tests need their own set-up (a fresh database per test).

use lib_bills::BillPlan;
use lib_budgets::Budget;
use lib_categories::{Category, is_uncategorised};
use lib_core::{BillPlanId, CategoryId, CategoryTypes, PayeeId, TransactionId};
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::error::Error;
use crate::fake::InMemoryCategoryDeletion;
use crate::fake::world::{
    DINING, FOOD, GROCERIES, GROCERIES_AND_DINING, GROCERIES_ONLY, GROCERY_BOX, LEASE, RENT,
    SALARY, UNCATEGORISED_INCOME, WATER, world,
};
use crate::interactor::DeleteCategory;
use crate::protocols::{CategoryDeletion, CategoryDeletionReads};

/// The rows a store holds, in an order two stores compare in: each by id.
#[derive(Debug, Clone, PartialEq)]
pub struct Stored {
    pub categories: Vec<Category>,
    pub transactions: Vec<Transaction>,
    pub bill_plans: Vec<BillPlan>,
    pub payees: Vec<Payee>,
    pub budgets: Vec<Budget>,
}

impl Stored {
    pub fn new(
        mut categories: Vec<Category>,
        mut transactions: Vec<Transaction>,
        mut bill_plans: Vec<BillPlan>,
        mut payees: Vec<Payee>,
        mut budgets: Vec<Budget>,
    ) -> Self {
        categories.sort_by_key(|category| category.id);
        transactions.sort_by_key(|transaction| transaction.id);
        bill_plans.sort_by_key(|plan| plan.id);
        payees.sort_by_key(|payee| payee.id);
        budgets.sort_by_key(|budget| budget.id);
        Self {
            categories,
            transactions,
            bill_plans,
            payees,
            budgets,
        }
    }

    fn of(rows: &InMemoryCategoryDeletion) -> Self {
        Self::new(
            rows.categories.categories.clone(),
            rows.transactions.transactions.clone(),
            rows.bills.plans.clone(),
            rows.payees.payees.clone(),
            rows.budgets.budgets.clone(),
        )
    }

    fn category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.iter().find(|category| category.id == id)
    }

    /// The Uncategorised Categories of `category_type`.
    fn uncategorised(&self, category_type: &CategoryTypes) -> Vec<&Category> {
        self.categories
            .iter()
            .filter(|category| {
                is_uncategorised(category) && category.category_type == *category_type
            })
            .collect()
    }

    /// Each Split's Category on Transaction `id`, in order.
    fn split_categories(&self, id: TransactionId) -> Vec<CategoryId> {
        self.transactions
            .iter()
            .find(|transaction| transaction.id == id)
            .map(|transaction| {
                transaction
                    .splits
                    .iter()
                    .map(|split| split.category_id)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn plan_category(&self, id: BillPlanId) -> Option<CategoryId> {
        self.bill_plans
            .iter()
            .find(|plan| plan.id == id)
            .map(|plan| plan.category_id)
    }

    fn payee_default(&self, id: PayeeId) -> Option<CategoryId> {
        self.payees
            .iter()
            .find(|payee| payee.id == id)
            .and_then(|payee| payee.default_category)
    }

    /// Whether any row names Category `id`.
    fn names(&self, id: CategoryId) -> bool {
        self.transactions
            .iter()
            .flat_map(|transaction| &transaction.splits)
            .any(|split| split.category_id == id)
            || self.bill_plans.iter().any(|plan| plan.category_id == id)
            || self
                .payees
                .iter()
                .any(|payee| payee.default_category == Some(id))
            || self
                .budgets
                .iter()
                .any(|budget| budget.limits.contains_key(&id))
    }
}

/// Builds the ports a contract check runs on.
pub trait DeletionFactory {
    type Ports: CategoryDeletion;

    /// Ports holding `rows`, as if stored and loaded.
    fn create(&mut self, rows: InMemoryCategoryDeletion) -> Self::Ports;

    /// What is stored now, read back as a restart would, without ending the writes made on the
    /// ports: what a port that committed for itself would already show.
    fn uncommitted(&mut self) -> Stored;

    /// Ends the writes made on `ports` with one commit, then reads back what is stored, as a
    /// restart would. A write that fails, a foreign key's among them, fails the check.
    fn stored(&mut self, ports: Self::Ports) -> Stored;

    /// Makes every Category delete from now on fail, as a store that refuses it would.
    fn break_category_deletes(&mut self, ports: &mut Self::Ports);

    /// Ends the writes made on `ports` and settles them, then reads back what is stored, with the
    /// number of failures reported.
    fn recover(&mut self, ports: Self::Ports) -> (Stored, usize);
}

/// The in-memory port is its own storage. It keeps a copy as of the last commit, which a failed
/// write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryCategoryDeletion>,
    broken: bool,
    failures: usize,
}

impl InMemoryFactory {
    fn commit(&mut self, ports: &InMemoryCategoryDeletion) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn read(&self) -> Stored {
        Stored::of(self.committed.as_ref().expect("create ran first"))
    }
}

impl DeletionFactory for InMemoryFactory {
    type Ports = InMemoryCategoryDeletion;

    fn create(&mut self, rows: InMemoryCategoryDeletion) -> InMemoryCategoryDeletion {
        self.committed = Some(rows.clone());
        rows
    }

    fn uncommitted(&mut self) -> Stored {
        self.read()
    }

    fn stored(&mut self, ports: InMemoryCategoryDeletion) -> Stored {
        self.commit(&ports);
        self.read()
    }

    fn break_category_deletes(&mut self, _ports: &mut InMemoryCategoryDeletion) {
        self.broken = true;
    }

    fn recover(&mut self, ports: InMemoryCategoryDeletion) -> (Stored, usize) {
        self.commit(&ports);
        (self.read(), std::mem::take(&mut self.failures))
    }
}

/// A delete stores the Category's Splits and Bill Plan on a new Expense Uncategorised, its Payee
/// defaults cleared and its Budget Limits dropped, then the Category gone; nothing names it, the
/// rows on other Categories stay, and every Transaction, Plan, Payee and Budget is still there.
pub fn a_delete_moves_its_rows_to_a_new_uncategorised<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let defaulting: Vec<PayeeId> = before
        .payees
        .iter()
        .filter(|payee| payee.default_category == Some(GROCERIES))
        .map(|payee| payee.id)
        .collect();
    let mut ports = factory.create(world());

    DeleteCategory { id: GROCERIES }
        .run(&mut ports)
        .expect("Groceries is a leaf");
    let stored = factory.stored(ports);

    let made = stored.uncategorised(&CategoryTypes::Expense);
    assert_eq!(made.len(), 1, "one Expense Uncategorised is made");
    let home = made.first().expect("one is made").id;
    assert_eq!(stored.category(GROCERIES), None);
    assert!(!stored.names(GROCERIES), "nothing names it");
    assert_eq!(stored.split_categories(GROCERIES_ONLY), [home]);
    assert_eq!(
        stored.split_categories(GROCERIES_AND_DINING),
        [home, DINING]
    );
    assert_eq!(stored.plan_category(GROCERY_BOX), Some(home));
    assert_eq!(stored.plan_category(LEASE), Some(RENT));
    assert!(!defaulting.is_empty());
    for payee in defaulting {
        assert_eq!(stored.payee_default(payee), None);
    }
    for (after, before) in stored.budgets.iter().zip(&before.budgets) {
        let mut expected = before.clone();
        expected.limits.remove(&GROCERIES);
        assert_eq!(after, &expected, "only Groceries' Limits go");
    }
    assert_eq!(stored.categories.len(), before.categories.len());
    assert_eq!(stored.transactions.len(), before.transactions.len());
    assert_eq!(stored.bill_plans.len(), before.bill_plans.len());
    assert_eq!(stored.payees.len(), before.payees.len());
    assert_eq!(stored.budgets.len(), before.budgets.len());
}

/// A Category whose rows move to an Uncategorised already held of its type makes none.
pub fn a_delete_reuses_the_uncategorised_of_its_type<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    DeleteCategory { id: SALARY }
        .run(&mut ports)
        .expect("Salary is a leaf");
    let stored = factory.stored(ports);

    assert_eq!(stored.categories.len(), before.categories.len() - 1);
    assert_eq!(stored.uncategorised(&CategoryTypes::Income).len(), 1);
    assert!(!stored.names(SALARY));
    assert!(
        stored
            .transactions
            .iter()
            .flat_map(|transaction| &transaction.splits)
            .filter(|split| split.category_id == UNCATEGORISED_INCOME)
            .count()
            == 2
    );
}

/// A Category no Split or Bill Plan names goes without making an Uncategorised; its Payee defaults
/// and Budget Limits still clear.
pub fn an_unused_delete_makes_no_uncategorised<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    DeleteCategory { id: WATER }
        .run(&mut ports)
        .expect("Water is a leaf");
    let stored = factory.stored(ports);

    assert_eq!(stored.category(WATER), None);
    assert!(!stored.names(WATER));
    assert!(stored.uncategorised(&CategoryTypes::Expense).is_empty());
    assert_eq!(stored.categories.len(), before.categories.len() - 1);
}

/// Nothing a delete writes is stored before the one commit that ends the call: the port commits
/// nothing for itself, so the bin's commit lands every row and the delete as one.
pub fn the_writes_wait_for_the_one_commit<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    DeleteCategory { id: GROCERIES }
        .run(&mut ports)
        .expect("Groceries is a leaf");

    assert_eq!(factory.uncommitted(), before, "nothing is stored yet");
    let stored = factory.stored(ports);
    assert_eq!(stored.category(GROCERIES), None);
    assert!(!stored.names(GROCERIES));
}

/// A delete whose Category delete fails stores none of it: no Uncategorised is made, every row
/// still names the Category, which stands, and the failure is reported.
pub fn a_failing_delete_lands_nothing<F: DeletionFactory>(factory: &mut F) {
    let before = Stored::of(&world());
    let mut ports = factory.create(world());

    factory.break_category_deletes(&mut ports);
    DeleteCategory { id: GROCERIES }
        .run(&mut ports)
        .expect("the port takes the write");
    let (stored, failures) = factory.recover(ports);

    assert_eq!(stored, before);
    assert!(failures > 0, "the failure is reported");
}

/// A Category not on the Ledger, a parent, and an Uncategorised whose rows would have nowhere to
/// go are refused by the port's own reads, before anything is written.
pub fn a_refusal_writes_nothing<F: DeletionFactory>(factory: &mut F) {
    let mut ports = factory.create(world());

    assert_eq!(
        DeleteCategory {
            id: CategoryId::seed(999)
        }
        .run(&mut ports),
        Err(Error::NotFound)
    );
    assert_eq!(
        DeleteCategory { id: FOOD }.run(&mut ports),
        Err(Error::HasChildren)
    );
    assert_eq!(
        DeleteCategory {
            id: UNCATEGORISED_INCOME
        }
        .run(&mut ports),
        Err(Error::UncategorisedInUse)
    );

    assert_eq!(factory.stored(ports), Stored::of(&world()));
}

/// Reads over [`world`]'s rows quote a Category with its Splits and the Budgets limiting it, and
/// nothing for a Category not held.
pub fn the_reads_quote_a_category_in_use(reads: &dyn CategoryDeletionReads) {
    let quote = |id| {
        reads
            .category_in_use(id)
            .map(|in_use| (in_use.category.name, in_use.splits, in_use.budgets))
    };

    assert_eq!(quote(GROCERIES), Some(("Groceries".to_string(), 2, 2)));
    assert_eq!(quote(RENT), Some(("Rent".to_string(), 1, 1)));
    assert_eq!(quote(FOOD), Some(("Food".to_string(), 0, 0)));
    assert_eq!(quote(CategoryId::seed(999)), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_moves_the_rows_to_a_new_uncategorised() {
        a_delete_moves_its_rows_to_a_new_uncategorised(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_reuses_the_uncategorised_of_the_type() {
        a_delete_reuses_the_uncategorised_of_its_type(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_makes_no_uncategorised_for_an_unused_category() {
        an_unused_delete_makes_no_uncategorised(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_waits_for_the_commit() {
        the_writes_wait_for_the_one_commit(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_lands_no_part_of_a_failing_delete() {
        a_failing_delete_lands_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_writes_nothing_for_a_refusal() {
        a_refusal_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn the_fake_reads_quote_a_category_in_use() {
        the_reads_quote_a_category_in_use(&world());
    }
}
