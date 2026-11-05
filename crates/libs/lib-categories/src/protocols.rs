//! The ports Categories reach storage and the other domains through, in the shape of `lib-payees'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! `lib-database`'s Categories adapter implements the repository (#858). The rest are ports onto
//! domains that aren't modules yet (#859): what names a Category ([`CategoryUsage`]), the Monthly
//! budget the Add and Edit Dialogs save into a Budget ([`CategoryBudget`]) and the handover of a
//! leaf's rows to its first child ([`CategoryHandover`]). A Client lends them to a Presenter's
//! confirm rather than the Presenter holding them, since the Desktop writes them only with the gpui
//! context that confirm runs in. Category delete re-points other domains' rows, so it is
//! `lib-category-deletion`'s (#1004, #1005). The Presenters ask for navigation through
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

/// How many rows in other domains name one Category, as the Edit Dialog quotes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryUse {
    pub splits: usize,
    /// The Budgets holding a Category Limit for it.
    pub budgets: usize,
}

/// What names a Category, for wording only: a read port, so `lib-categories` needn't depend on
/// the Transactions or Budgets crates. Counted from what the Client holds, and 0 when it can't
/// count.
pub trait CategoryUsage {
    fn category_usage(&self, id: CategoryId) -> CategoryUse;

    /// What stops a top-level Category's type changing: the Budgets and Bill Plans using any
    /// Category in `subtree` (the Category and everything nested under it).
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers;
}

/// The rows that keep a Category's subtree from changing type. A Budget counts when it is active
/// and a Budget Amount of a Category in the subtree is in effect this month or starts later, so a
/// chain Stopped before this month and an archived Budget don't. A Bill Plan counts whether or
/// not it is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeChangeBlockers {
    pub budgets: usize,
    pub bill_plans: usize,
}

impl TypeChangeBlockers {
    pub fn is_empty(&self) -> bool {
        self.budgets == 0 && self.bill_plans == 0
    }
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

/// Where a Category that is about to gain a child is budgeted, for the Dialogs' notice: the
/// active Budgets that will Stop it, and the month they Stop it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentStop {
    pub budgets: usize,
    /// The calendar month (1–12) the Stop starts, the current one.
    pub month: u32,
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

    /// Where `category` is budgeted this month, in every active Budget, or `None` when nowhere.
    /// The Dialogs ask it of a chosen parent to warn that a child will Stop its budget.
    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop>;

    /// Stops `parent`'s budget from this month in every active Budget, once it has gained its first
    /// child (a parent only rolls up). Closed months keep their amounts, and a Category budgeted
    /// nowhere writes nothing.
    fn stop_new_parent(&mut self, parent: CategoryId);
}

/// What a leaf Category holds that moves to its first child: the Splits filed under it, the Bill
/// Plans naming it and the Payees defaulting to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Holdings {
    pub splits: usize,
    pub bill_plans: usize,
    pub payee_defaults: usize,
}

impl Holdings {
    pub fn is_empty(&self) -> bool {
        self.splits == 0 && self.bill_plans == 0 && self.payee_defaults == 0
    }
}

/// The handover of a leaf's rows to its first child. Only a leaf is assigned to a Split, a Bill
/// Plan or a Payee default, so a Category that gains its first child, added or moved in, passes
/// them to it. Those rows belong to other domains, so the Interactors reach them through this
/// port, which a Client answers over their stores in the input's one commit, as it does
/// [`CategoryBudget`].
pub trait CategoryHandover {
    /// What `category` holds now, for the Dialogs' notice.
    fn holdings(&self, category: CategoryId) -> Holdings;

    /// Points every Split, Bill Plan and Payee default naming `from` at `to` instead. Writes
    /// nothing when `from` holds nothing.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId);
}

/// Everything an Add or Edit save reaches beyond the Categories: the Monthly budget and the
/// handover. One trait, so a Client lends its one set of ports once.
pub trait CategorySavePorts: CategoryBudget + CategoryHandover + CategoryUsage {}

impl<T: CategoryBudget + CategoryHandover + CategoryUsage + ?Sized> CategorySavePorts for T {}

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
}
