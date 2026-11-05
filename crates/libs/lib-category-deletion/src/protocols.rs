//! The port Category delete writes through, and the reads its Dialog shows.
//!
//! [`CategoryDeletion`] is the module's one atomic port: a Category goes with the re-pointing of
//! every row that names it, written together or not at all. It is synchronous (ADR-0046): an
//! adapter serves its reads from memory and queues every write in the batch every adapter shares,
//! so the bin's one commit after the call lands them as one transaction, and a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! It serves the reads the Interactor checks as owned values, rather than the Interactor taking
//! the five domains' ports beside it: the adapter holds stores those modules share, so an
//! Interactor given them all would borrow one twice (#779).
//!
//! What the Dialog only shows, the Category with its Splits and Budgets, it reads through
//! [`CategoryDeletionReads`] once, when it opens: it is modal, so nothing changes it under it.
//!
//! The Dialog closes through the shared `lib_viper::dialog::DialogRouter` with `lib-categories`'
//! `CategoriesOutcome`, as every Categories Dialog does, so the Categories page hears of a delete
//! as it would of its own.

use std::cell::RefCell;
use std::rc::Rc;

use lib_bills::BillPlan;
use lib_budgets::Budget;
use lib_categories::Category;
use lib_core::CategoryId;
use lib_payees::Payee;
use lib_transactions::Transaction;

/// Everything a delete writes, as one write, each row whole as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryRemoval {
    /// The Uncategorised Category the moved rows go to, when the delete made it.
    pub home: Option<Category>,
    /// The Transactions whose Splits named the Category.
    pub transactions: Vec<Transaction>,
    /// The Bill Plans that named it.
    pub bill_plans: Vec<BillPlan>,
    /// The Payees that defaulted to it.
    pub payees: Vec<Payee>,
    /// The Budgets that held a Limit for it.
    pub budgets: Vec<Budget>,
    /// The Category deleted once nothing names it.
    pub deleted: CategoryId,
}

/// The rows Category delete checks, and the one write that lands it.
pub trait CategoryDeletion {
    /// The Category tree, which the Category must be a leaf of and the Uncategorised home is found
    /// in.
    fn categories(&self) -> Vec<Category>;

    /// The Transactions with a Split naming Category `id`, in the store's order.
    fn transactions_using(&self, id: CategoryId) -> Vec<Transaction>;

    /// The Bill Plans naming Category `id`, active or not.
    fn bill_plans_using(&self, id: CategoryId) -> Vec<BillPlan>;

    /// The Payees whose Default category is `id`.
    fn payees_defaulting_to(&self, id: CategoryId) -> Vec<Payee>;

    /// The Budgets, archived or not, holding a Limit for Category `id`.
    fn budgets_limiting(&self, id: CategoryId) -> Vec<Budget>;

    /// Writes `removal` as one write, in the foreign keys' order: the Uncategorised home, the
    /// Transactions, the Bill Plans, the Payees, the Budgets (each whole), then the Category's
    /// delete, which nothing may name by then. All land or none do.
    fn remove(&mut self, removal: CategoryRemoval);
}

/// Those of `transactions` with a Split naming Category `id`, in order: what every store answers
/// [`CategoryDeletion::transactions_using`] with.
pub fn transactions_using(transactions: Vec<Transaction>, id: CategoryId) -> Vec<Transaction> {
    transactions
        .into_iter()
        .filter(|transaction| {
            transaction
                .splits
                .iter()
                .any(|split| split.category_id == id)
        })
        .collect()
}

/// Those of `plans` naming Category `id`: what every store answers
/// [`CategoryDeletion::bill_plans_using`] with.
pub fn bill_plans_using(plans: Vec<BillPlan>, id: CategoryId) -> Vec<BillPlan> {
    plans
        .into_iter()
        .filter(|plan| plan.category_id == id)
        .collect()
}

/// Those of `payees` defaulting to Category `id`: what every store answers
/// [`CategoryDeletion::payees_defaulting_to`] with.
pub fn payees_defaulting_to(payees: Vec<Payee>, id: CategoryId) -> Vec<Payee> {
    payees
        .into_iter()
        .filter(|payee| payee.default_category == Some(id))
        .collect()
}

/// Those of `budgets` holding a Limit for Category `id`: what every store answers
/// [`CategoryDeletion::budgets_limiting`] with.
pub fn budgets_limiting(budgets: Vec<Budget>, id: CategoryId) -> Vec<Budget> {
    budgets
        .into_iter()
        .filter(|budget| budget.limits.contains_key(&id))
        .collect()
}

/// The deletion port as the Delete category Dialog holds it, shared with the Client that commits
/// its writes. Single-threaded, as both Clients' UI threads are.
pub type SharedCategoryDeletion = Rc<RefCell<dyn CategoryDeletion>>;

/// A Category as the Dialog quotes it, with the rows using it.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryInUse {
    pub category: Category,
    /// The Splits naming it, which move to Uncategorised.
    pub splits: usize,
    /// The Budgets holding a Limit for it.
    pub budgets: usize,
}

/// What the Delete category Dialog shows, read once when it opens.
pub trait CategoryDeletionReads {
    /// Category `id` and what uses it, or `None` when it is gone.
    fn category_in_use(&self, id: CategoryId) -> Option<CategoryInUse>;
}

/// What a delete did, for its Toast and the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDeleted {
    pub id: CategoryId,
    /// The Category's name, read before it went.
    pub name: String,
    /// The Splits moved to Uncategorised.
    pub splits_moved: usize,
    /// The Bill Plans moved to Uncategorised.
    pub bill_plans_moved: usize,
    /// The Payees that no longer default to it.
    pub payee_defaults_cleared: usize,
    /// The Budgets that dropped its Limits.
    pub budgets_touched: usize,
}
