//! One Presenter per Categories View: the Settings page and each of its Dialogs (Add and Edit
//! share the form). Each takes events through `handle` and builds a plain-data view model through
//! `view_model`, its text already localised from [`crate::msg`], so a View only draws. They share
//! one repository ([`SharedCategories`](crate::protocols::SharedCategories)) and ask for
//! navigation through the [`CategoriesRouter`](crate::protocols::CategoriesRouter) the Client
//! passes in. A Dialog's confirm is lent the port that writes through the Client's Ledger.

mod categories_page;
mod category_form;
mod delete_category;

pub use categories_page::{
    CategoriesPageEvent, CategoriesPagePresenter, CategoriesPageViewModel, CategoryColumns,
    CategoryLine, CategoryTree,
};
pub use category_form::{
    BudgetField, CategoryField, CategoryFormEvent, CategoryFormPresenter, CategoryFormViewModel,
};
pub use delete_category::{DeleteCategoryEvent, DeleteCategoryPresenter, DeleteCategoryViewModel};

use crate::protocols::{MonthlyBudgetRefusal, ReassignRefusal};
use crate::{CategoryError, SaveCategoryRefusal, msg};

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// A refused Add or Edit as the form shows it.
fn save_refusal_text(refusal: &SaveCategoryRefusal) -> String {
    match refusal {
        SaveCategoryRefusal::Category(error) => match error {
            CategoryError::NotFound => msg::categories_refusal_gone(),
            CategoryError::ParentNotFound => msg::categories_refusal_parent_gone(),
            CategoryError::TypeMismatch => msg::categories_refusal_type_mismatch(),
            CategoryError::DepthExceeded => msg::categories_refusal_too_deep(),
            CategoryError::CycleDetected => msg::categories_refusal_cycle(),
        },
        SaveCategoryRefusal::Budget(refusal) => match refusal {
            MonthlyBudgetRefusal::NotAnExpenseLeaf => msg::categories_refusal_budget_not_a_leaf(),
            MonthlyBudgetRefusal::ReadOnly => msg::categories_refusal_budget_read_only(),
            MonthlyBudgetRefusal::Unavailable => msg::categories_refusal_budget_unavailable(),
        },
    }
}

/// A refused delete as the Delete Dialog shows it.
fn reassign_refusal_text(refusal: ReassignRefusal) -> String {
    match refusal {
        ReassignRefusal::NotFound => msg::categories_refusal_gone(),
        ReassignRefusal::HasChildren => msg::categories_refusal_has_children(),
        ReassignRefusal::UncategorisedInUse => msg::categories_refusal_uncategorised_in_use(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::InMemoryCategories;
    use crate::protocols::SharedCategories;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The stub seed, shared, with the handle the test reads it back through.
    pub fn stub() -> (Rc<RefCell<InMemoryCategories>>, SharedCategories) {
        locale();
        InMemoryCategories::stub().shared()
    }
}
