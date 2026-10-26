//! The ports Categories reach storage and the other domains through, in the shape of `lib-payees'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! `lib-database`'s Categories adapter implements the repository (#858). The rest are ports onto
//! domains that aren't modules yet (#859): what names a Category ([`CategoryUsage`]), the Monthly
//! budget the Add and Edit Dialogs save into a Budget ([`CategoryBudget`]), and Category delete,
//! which re-points the rows naming it ([`CategoryReassign`], Q6). A Client answers the last two
//! through its Ledger until `lib-reassign` (#813) and Budgets (#811), and lends them to a
//! Presenter's confirm rather than the Presenter holding them, since the Desktop writes its Ledger
//! only with the gpui context that confirm runs in. The Presenters ask for navigation through
//! [`CategoriesRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{CategoryId, Money};
use lib_toast::Notify;

use crate::Category;

/// The Categories on the Ledger.
pub trait CategoryRepository {
    /// Every Category in tree order: each parent before its children, depth first, and siblings in
    /// the order they were added (see [`tree_order`](crate::tree_order)).
    fn categories(&self) -> Vec<Category>;

    fn category(&self, id: CategoryId) -> Option<Category> {
        self.categories()
            .into_iter()
            .find(|category| category.id == id)
    }

    /// Writes `category` over the row with its id, keeping that row's place among its siblings, or
    /// as a new row after every row already there.
    fn upsert_category(&mut self, category: Category);
}

/// The Categories as the page and its Dialogs hold them: one repository, shared, so a Category a
/// Dialog adds is on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI
/// threads are.
pub type SharedCategories = Rc<RefCell<dyn CategoryRepository>>;

/// How many rows in other domains name one Category, as the Edit and Delete Dialogs quote them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryUse {
    pub splits: usize,
    /// The Budgets holding a Category Limit for it.
    pub budgets: usize,
}

/// What names a Category, for wording only: a read port, so `lib-categories` needn't depend on
/// the Transactions or Budgets crates. Counted from what the Client holds, and 0 when it can't
/// count, since a delete's refusal is [`CategoryReassign`]'s.
pub trait CategoryUsage {
    fn category_usage(&self, id: CategoryId) -> CategoryUse;
}

/// The Monthly budget field as a Dialog opens it: the amount this month, and the Budget's name
/// when it is archived and so read-only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MonthlyBudget {
    pub amount: Option<Money>,
    pub archived: Option<String>,
}

/// Why a Monthly budget can't be saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MonthlyBudgetRefusal {
    /// Only a leaf Expense Category takes a budget amount.
    #[error("only a leaf expense category can be budgeted")]
    NotAnExpenseLeaf,
    /// The Budget is archived, or the month closed.
    #[error("the budget is read-only")]
    ReadOnly,
    /// The Budget has gone, or refused for a reason no Dialog reaches.
    #[error("the budget is unavailable")]
    Unavailable,
}

/// The one Budget the Add and Edit Dialogs read and save a Category's Monthly budget in (the
/// Desktop's Personal spending Budget), held by the Budgets domain. A save checks before it
/// writes, so a refused budget leaves the Category unwritten too.
pub trait CategoryBudget {
    /// The field for `category` (`None` while adding) in `categories`: a parent's is the sum of its
    /// leaves'.
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget;

    /// Whether `amount` (`None` stops the budget) would save for `category` in `categories`, the
    /// tree as the save leaves it. Writes nothing.
    fn check_monthly_budget(
        &self,
        categories: &[Category],
        category: CategoryId,
        amount: Option<&Money>,
    ) -> Result<(), MonthlyBudgetRefusal>;

    /// Saves `amount` for `category`, once its Category is written. An unchanged amount writes
    /// nothing.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal>;
}

/// What a Category delete did, for its Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryReassigned {
    /// The Category's name, read before it went.
    pub name: String,
    /// The Splits moved to Uncategorised.
    pub splits_moved: usize,
}

/// Why a Category delete was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReassignRefusal {
    #[error("category not found")]
    NotFound,
    /// Only leaves are deleted; a parent's children go first.
    #[error("delete or move its children first")]
    HasChildren,
    /// Uncategorised is where a deleted Category's rows go, so it can't go while anything uses it.
    #[error("Uncategorised can't be deleted while anything uses it")]
    UncategorisedInUse,
}

/// Category delete (Q6): removes a leaf Category once its Splits and Bill Plans are moved to the
/// Uncategorised Category of its type (made if needed), the Payees defaulting to it are cleared and
/// the Budgets drop its Limits, all in the one commit of the input. The Delete Dialog's Presenter
/// confirms through it; it re-points other domains' rows, so it is no `lib-categories` Interactor.
/// The pattern Tags and Inventory repeat, and the port `lib-reassign` (#813) answers.
pub trait CategoryReassign {
    fn delete_category(&mut self, id: CategoryId) -> Result<CategoryReassigned, ReassignRefusal>;
}

/// A Categories Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoriesDialog {
    /// Add, at top level or under `parent`.
    AddCategory {
        parent: Option<CategoryId>,
    },
    EditCategory(CategoryId),
    DeleteCategory(CategoryId),
}

/// What a Dialog did to a Category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryChange {
    /// Added or edited.
    Saved,
    Deleted,
}

/// What a confirmed Categories Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriesOutcome {
    pub change: CategoryChange,
    pub id: CategoryId,
    /// The Category's name as written, read before a delete removes it.
    pub name: String,
}

/// Navigation out of the Categories Presenters, implemented by each Client. The Client passes it
/// into each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait CategoriesRouter: Notify {
    /// Builds `dialog`'s Presenter and hosts its View. Returns whether it opened: a Dialog on a
    /// Category removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: CategoriesDialog) -> bool;

    /// Closes the open Categories Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`CategoriesPageEvent::DialogClosed`].
    ///
    /// [`CategoriesPageEvent::DialogClosed`]: crate::presenter::CategoriesPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<CategoriesOutcome>);
}
