//! One Presenter per Categories View: the Settings page and its Add and Edit Dialogs, which share
//! the form. The Delete category Dialog's is `lib-category-deletion`'s (#1004). Each takes events
//! through `handle` and builds a plain-data view model through `view_model`, its text already localised from [`crate::msg`], so a View only draws. They share
//! one repository ([`SharedCategories`](crate::protocols::SharedCategories)) and ask for
//! navigation through the [`CategoriesRouter`](crate::protocols::CategoriesRouter) the Client
//! passes in. The form holds the ports onto the other domains its save reaches from the moment it opens (ADR-0056).

mod categories_page;
mod category_form;

pub use categories_page::{
    CategoriesPageEvent, CategoriesPagePresenter, CategoriesPageViewModel, CategoryColumns,
    CategoryLine, CategoryTree,
};
pub use category_form::{
    BudgetField, CategoryField, CategoryFormEvent, CategoryFormPresenter, CategoryFormViewModel,
};

use lib_core::CategoryTypes;
use lib_viper::view::count;

use crate::protocols::TypeChangeBlockers;
use crate::{Error, msg};

/// A refused Add or Edit as the form shows it.
fn save_refusal_text(refusal: &Error) -> String {
    match refusal {
        Error::NotFound => msg::categories_refusal_gone(),
        Error::ParentNotFound => msg::categories_refusal_parent_gone(),
        Error::TypeMismatch => msg::categories_refusal_type_mismatch(),
        Error::DepthExceeded => msg::categories_refusal_too_deep(),
        Error::CycleDetected => msg::categories_refusal_cycle(),
        Error::UncategorisedFixed => msg::categories_refusal_uncategorised_fixed(),
        Error::LeafHoldsRows { leaf, held } => {
            let mut parts = Vec::new();
            if held.splits > 0 {
                parts.push(msg::categories_holds_splits());
            }
            if held.bill_plans > 0 {
                parts.push(msg::categories_holds_bill_plans());
            }
            if held.payee_defaults > 0 {
                parts.push(msg::categories_holds_payee_defaults());
            }
            let rows = match parts.as_slice() {
                [first, second, third] => msg::categories_handover_three(first, second, third),
                [first, second] => msg::categories_handover_two(first, second),
                [one] => one.clone(),
                _ => String::new(),
            };
            msg::categories_refusal_leaf_holds_rows(leaf, &rows)
        }
        Error::TypeInUse {
            category,
            to,
            blockers,
        } => type_in_use_text(category, to, *blockers),
        Error::NotAnExpenseLeaf => msg::categories_refusal_budget_not_a_leaf(),
        Error::BudgetReadOnly => msg::categories_refusal_budget_read_only(),
        Error::BudgetUnavailable => msg::categories_refusal_budget_unavailable(),
    }
}

/// Why `category` can't become `to`: the notice under Type and the refusal on save say the same.
fn type_in_use_text(category: &str, to: &CategoryTypes, blockers: TypeChangeBlockers) -> String {
    let mut parts = Vec::new();
    if blockers.budgets > 0 {
        parts.push(msg::categories_type_in_use_budgets(count(blockers.budgets)));
    }
    if blockers.bill_plans > 0 {
        parts.push(msg::categories_type_in_use_bill_plans(count(
            blockers.bill_plans,
        )));
    }
    let uses = match parts.as_slice() {
        [first, second] => msg::categories_handover_two(first, second),
        [one] => one.clone(),
        _ => String::new(),
    };
    let kind = lib_locale::Label::label(to);
    if blockers.budgets + blockers.bill_plans == 1 {
        msg::categories_type_in_use_single(category, &kind, &uses)
    } else {
        msg::categories_type_in_use(category, &kind, &uses)
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

    /// The sample seed, shared, with the handle the test reads it back through.
    pub fn sample() -> (Rc<RefCell<InMemoryCategories>>, SharedCategories) {
        locale();
        InMemoryCategories::sample().shared()
    }
}
