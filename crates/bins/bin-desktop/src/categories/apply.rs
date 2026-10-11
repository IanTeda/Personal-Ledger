//! The Categories Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Categories, Transactions and Budgets stores. When
//! `LedgerStore` replaces the per-domain stores only [`CategoriesApply`] changes.

use std::fmt;

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_core::{CategoryTypes, Money};
use lib_transactions::Transaction;

use super::{
    CategoryError,
    form::{CategoriesDialog, CategoryForm},
};
use crate::{
    budgets,
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    transactions::{TransactionsStore, edit_transactions},
    view::{budgets::state::BudgetsStore, categories::state::CategoriesStore},
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
    Category(CategoryError),
    /// The Monthly budget write the dialog carries was refused.
    Budget(String),
}

impl fmt::Display for CategoryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Category(error) => error.fmt(f),
            Self::Budget(reason) => f.write_str(reason),
        }
    }
}

impl From<CategoryError> for CategoryRefusal {
    fn from(error: CategoryError) -> Self {
        Self::Category(error)
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

/// Runs [`CategoryCommand`]s against the Categories store, re-pointing Splits in the Transactions
/// store and writing Monthly budgets into the Budgets store.
pub struct CategoriesApply {
    pub categories: Entity<CategoriesStore>,
    pub transactions: Entity<TransactionsStore>,
    pub budgets: Entity<BudgetsStore>,
    pub today: NaiveDate,
}

impl CategoriesApply {
    /// Categories 5c's write on a saved dialog: the Monthly budget text as an Onward amount from
    /// the current month in the Personal spending Budget, or a Stop when `clears` and it is
    /// blank. A locked field (a parent's rollup, an archived Budget) writes nothing.
    fn save_budget(
        &self,
        category_id: u32,
        form: &CategoryForm,
        clears: bool,
        cx: &mut App,
    ) -> Result<(), CategoryRefusal> {
        if form.budget_lock.is_some() {
            return Ok(());
        }
        let amount = if form.budget.is_blank() {
            if !clears {
                return Ok(());
            }
            None
        } else {
            match form.budget.text().trim().parse::<Money>() {
                Ok(amount) => Some(amount),
                Err(_) => return Ok(()),
            }
        };
        let categories = self.categories.read(cx).categories().to_vec();
        let today = self.today;
        self.budgets
            .update(cx, |store, cx| {
                store.mutate(cx, |budgets| {
                    budgets.set_monthly_limit(
                        budgets::PERSONAL_SPENDING_ID,
                        &categories,
                        category_id,
                        amount,
                        today,
                    )
                })
            })
            .map(|_| ())
            .map_err(|error| CategoryRefusal::Budget(error.to_string()))
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
                let category_type = form.category_type.clone().unwrap_or(CategoryTypes::Expense);
                let category_id = self.categories.update(cx, |store, cx| {
                    store.insert(
                        cx,
                        form.name.text().trim().to_string(),
                        form.parent_id,
                        category_type,
                    )
                })?;
                self.save_budget(category_id, &form, false, cx)?;
                Ok(CategoryOutcome::Saved)
            }
            CategoryCommand::Edit(id, form) => {
                let name = form.name.text().trim().to_string();
                self.categories
                    .update(cx, |store, cx| store.edit(cx, id, name))?;
                // `None` moves it to top level when the parent was cleared.
                self.categories
                    .update(cx, |store, cx| store.move_to(cx, id, form.parent_id))?;
                self.save_budget(id, &form, true, cx)?;
                Ok(CategoryOutcome::Saved)
            }
            CategoryCommand::Delete(id) => {
                // In order: check and get the Uncategorised Category, re-point the Splits, drop
                // the budget, then drop the Category.
                let uncategorised = self
                    .categories
                    .update(cx, |store, cx| store.prepare_delete(cx, id))?;
                let name = self
                    .categories
                    .read(cx)
                    .categories()
                    .iter()
                    .find(|category| category.id == id)
                    .map(|category| category.name.clone())
                    .unwrap_or_default();
                let moved = edit_transactions(&self.transactions, cx, |transactions| {
                    move_category_splits(transactions, id, uncategorised)
                });
                self.budgets.update(cx, |store, cx| {
                    store.mutate(cx, |budgets| budgets.remove_category(id))
                });
                self.categories
                    .update(cx, |store, cx| store.delete(cx, id))?;
                Ok(CategoryOutcome::Deleted { name, moved })
            }
        }
    }
}

/// Re-points every Split on category `from` to `to`, returning how many moved. Runs on the
/// Transactions store, so it borrows no Category or Budget state.
pub fn move_category_splits(transactions: &mut [Transaction], from: u32, to: u32) -> i64 {
    let mut moved = 0_i64;
    for split in transactions.iter_mut().flat_map(|t| t.splits.iter_mut()) {
        if split.category_id == from {
            split.category_id = to;
            moved += 1;
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{
        categories::form::DeleteCategoryForm, chrome::dialog_apply::confirm, form::field::TextField,
    };

    fn apply(cx: &mut TestAppContext) -> CategoriesApply {
        let categories = cx.new(|_| CategoriesStore::new(lib_categories::default_categories()));
        let transactions = cx.new(|_| TransactionsStore::new(Vec::new()));
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let budgets = cx.new(|_| {
            BudgetsStore::new(budgets::default_budgets(
                &crate::accounts::default_accounts(),
                &lib_categories::default_categories(),
                today,
            ))
        });
        CategoriesApply {
            categories,
            transactions,
            budgets,
            today,
        }
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
                if form.error == Some(CategoryRefusal::Category(CategoryError::NotFound))
        ));
    }
}
