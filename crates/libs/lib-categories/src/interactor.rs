//! One Interactor per Categories intent, each a request value whose `run` takes the ports it needs.
//! Every rule is the Entity's and is checked before anything is written, so a refusal leaves the
//! ports untouched. Delete re-points other domains' rows, so it is a port of its own,
//! [`CategoryReassign`](crate::protocols::CategoryReassign), rather than an Interactor (#847, Q6).

mod add_category;
mod edit_category;
mod move_category;
mod rename_category;

pub use add_category::AddCategory;
pub use edit_category::EditCategory;
pub use move_category::MoveCategory;
pub use rename_category::RenameCategory;

use lib_core::{CategoryId, Money};

use crate::Category;
use crate::protocols::{CategoryBudget, MonthlyBudgetRefusal};

/// What an Add or Edit does to the Category's Monthly budget.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BudgetChange {
    /// Leaves the Budget as it is: a blank Add, or a read-only field.
    #[default]
    Keep,
    /// An Onward amount from this month.
    Set(Money),
    /// Stops the budget from this month: an Edit whose field was cleared.
    Stop,
}

impl BudgetChange {
    /// The amount to save, `Some(None)` to stop, or `None` to leave the Budget alone.
    fn amount(&self) -> Option<Option<&Money>> {
        match self {
            Self::Keep => None,
            Self::Set(amount) => Some(Some(amount)),
            Self::Stop => Some(None),
        }
    }

    /// Whether the change would save for `id` in `rows`, the tree as the save leaves it.
    fn check(
        &self,
        budget: &dyn CategoryBudget,
        rows: &[Category],
        id: CategoryId,
    ) -> Result<(), MonthlyBudgetRefusal> {
        match self.amount() {
            Some(amount) => budget.check_monthly_budget(rows, id, amount),
            None => Ok(()),
        }
    }

    /// Saves the change for `id`, once the Category is written. It was checked first, so a refusal
    /// here is the Budget changing under the save, which is logged rather than undoing the
    /// Category.
    fn save(&self, budget: &mut dyn CategoryBudget, id: CategoryId) {
        if let Some(amount) = self.amount()
            && let Err(refusal) = budget.set_monthly_budget(id, amount.cloned())
        {
            tracing::warn!(category_id = %id, %refusal, "A checked Monthly budget was refused");
        }
    }
}
