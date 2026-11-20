//! The Categories Refusals (`GLOSSARY.md`): one variant per rule a Category write, or the Monthly
//! budget it carries, enforces. The `#[error]` text is for logs; the person sees the Presenter's
//! `msg::` Message.

use lib_core::CategoryTypes;

use crate::protocols::{Holdings, TypeChangeBlockers};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Attempted to nest a category deeper than the 3-level cap.
    #[error("categories nest at most three levels deep")]
    DepthExceeded,
    /// Attempted to make a category its own parent (directly or via a cycle).
    #[error("a category can't be its own parent")]
    CycleDetected,
    /// Attempted to move an Expense to Income (or vice versa) or parent has different type.
    #[error("the parent category is a different type")]
    TypeMismatch,
    /// The Category is no longer on the Ledger.
    #[error("category not found")]
    NotFound,
    /// The chosen parent is no longer on the Ledger.
    #[error("parent category not found")]
    ParentNotFound,
    /// Attempted to rename, move, retype or nest under Uncategorised, which is fixed.
    #[error("Uncategorised is fixed")]
    UncategorisedFixed,
    /// A Category with children can't move under a leaf that holds Splits, Bill Plans or Payee
    /// defaults: they would have no child to take them.
    #[error("{leaf} holds rows; add a Category under it first")]
    LeafHoldsRows { leaf: String, held: Holdings },
    /// A top-level Category can't change type while Budgets or Bill Plans use its Categories.
    #[error("{category} can't change type while its categories are in use")]
    TypeInUse {
        category: String,
        to: CategoryTypes,
        blockers: TypeChangeBlockers,
    },
    /// Only a leaf Expense Category takes a Monthly budget amount.
    #[error("only a leaf expense category can be budgeted")]
    NotAnExpenseLeaf,
    /// The Monthly budget's Budget is archived, or the month closed.
    #[error("the budget is read-only")]
    BudgetReadOnly,
    /// The Monthly budget's Budget has gone, or refused for a reason no Dialog reaches.
    #[error("the budget is unavailable")]
    BudgetUnavailable,
}

pub type Result<T> = std::result::Result<T, Error>;
