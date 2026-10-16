//! The Categories Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger as Ledger commands.

use std::fmt;

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_core::{CategoryTypes, Money};
use lib_ledger::{
    AddCategory, DeleteCategory, DeleteCategoryRefusal, EditCategory, MonthlyBudget,
    SaveCategoryRefusal,
};

use super::form::{CategoriesDialog, CategoryForm};
use crate::{
    budgets,
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    ledger_store::LedgerStore,
};

/// What confirming a Categories Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryCommand {
    Add(CategoryForm),
    Edit(u32, CategoryForm),
    Delete(u32),
}

/// What a landed [`CategoryCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryOutcome {
    Saved,
    /// Deleted: the name, read before the write removes it, and how many Splits moved to
    /// Uncategorised.
    Deleted {
        name: String,
        moved: i64,
    },
}

/// Why a Categories command didn't land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryRefusal {
    /// An Add or Edit the tree rules or the Monthly budget it carried refused.
    Save(SaveCategoryRefusal),
    Delete(DeleteCategoryRefusal),
}

impl fmt::Display for CategoryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Save(refusal) => refusal.fmt(f),
            Self::Delete(refusal) => refusal.fmt(f),
        }
    }
}

impl DialogForm for CategoriesDialog {
    type Command = CategoryCommand;
    type Refusal = CategoryRefusal;

    fn command(&self) -> Result<CategoryCommand, FormError> {
        match self {
            Self::Add { form, .. } if form.is_valid() => Ok(CategoryCommand::Add(form.clone())),
            Self::Edit(id, form) if form.is_valid() => Ok(CategoryCommand::Edit(*id, form.clone())),
            Self::Delete(id, form) if form.is_valid() => Ok(CategoryCommand::Delete(*id)),
            _ => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: CategoryRefusal) {
        match self {
            Self::Add { form, .. } | Self::Edit(_, form) => form.error = Some(refusal),
            Self::Delete(_, form) => form.error = Some(refusal),
        }
    }
}

/// Runs [`CategoryCommand`]s against the Ledger. `today` anchors the Monthly budget write.
pub struct CategoriesApply {
    pub ledger: Entity<LedgerStore>,
    pub today: NaiveDate,
}

impl CategoriesApply {
    /// Categories 5c's write on a saved dialog: the Monthly budget text as an Onward amount from
    /// the current month in the Personal spending Budget, or a Stop when `clears` and it is
    /// blank. A locked field (a parent's rollup, an archived Budget) or text that isn't an amount
    /// writes nothing.
    fn monthly_budget(&self, form: &CategoryForm, clears: bool) -> Option<MonthlyBudget> {
        if form.budget_lock.is_some() {
            return None;
        }
        let limit = if form.budget.is_blank() {
            if !clears {
                return None;
            }
            None
        } else {
            Some(form.budget.text().trim().parse::<Money>().ok()?)
        };
        Some(MonthlyBudget {
            budget_id: budgets::PERSONAL_SPENDING_ID,
            today: self.today,
            limit,
        })
    }
}

impl Apply<CategoryCommand> for CategoriesApply {
    type Outcome = CategoryOutcome;
    type Refusal = CategoryRefusal;

    fn apply(
        &mut self,
        command: CategoryCommand,
        cx: &mut App,
    ) -> Result<CategoryOutcome, CategoryRefusal> {
        match command {
            CategoryCommand::Add(form) => {
                let command = AddCategory {
                    name: form.name.text().trim().to_string(),
                    parent_id: form.parent_id,
                    category_type: form.category_type.clone().unwrap_or(CategoryTypes::Expense),
                    monthly: self.monthly_budget(&form, false),
                };
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, command))
                    .map_err(CategoryRefusal::Save)?;
                Ok(CategoryOutcome::Saved)
            }
            CategoryCommand::Edit(id, form) => {
                // `None` moves it to top level when the parent was cleared.
                let command = EditCategory {
                    id,
                    name: form.name.text().trim().to_string(),
                    parent_id: form.parent_id,
                    monthly: self.monthly_budget(&form, true),
                };
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, command))
                    .map_err(CategoryRefusal::Save)?;
                Ok(CategoryOutcome::Saved)
            }
            CategoryCommand::Delete(id) => {
                let deleted = self
                    .ledger
                    .update(cx, |store, cx| store.apply(cx, DeleteCategory(id)))
                    .map_err(CategoryRefusal::Delete)?;
                Ok(CategoryOutcome::Deleted {
                    name: deleted.name,
                    moved: i64::try_from(deleted.splits_moved).unwrap_or(i64::MAX),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{
        categories::form::DeleteCategoryForm, chrome::dialog_apply::confirm, form::field::TextField,
    };

    fn apply(cx: &mut TestAppContext) -> CategoriesApply {
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let ledger = test_ledger(
            cx,
            LedgerParts {
                categories: lib_categories::default_categories(),
                budgets: budgets::default_budgets(
                    &lib_accounts::default_accounts(),
                    &lib_categories::default_categories(),
                    today,
                ),
                ..LedgerParts::default()
            },
        );
        CategoriesApply { ledger, today }
    }

    #[gpui::test]
    fn adding_a_blank_name_is_not_a_command_and_a_named_one_lands(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = CategoriesDialog::Add {
            parent_id: None,
            form: CategoryForm::default(),
        };
        assert_eq!(dialog.command(), Err(FormError));

        if let CategoriesDialog::Add { form, .. } = &mut dialog {
            form.name = TextField::new("Pet care");
        }
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, Some(CategoryOutcome::Saved));
    }

    #[gpui::test]
    fn a_vanished_category_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = CategoriesDialog::Delete(9999, DeleteCategoryForm::new(""));
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, None);
        assert!(matches!(
            &dialog,
            CategoriesDialog::Delete(_, form)
                if form.error == Some(CategoryRefusal::Delete(DeleteCategoryRefusal::NotFound))
        ));
    }
}
