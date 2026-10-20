//! The Categories Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use std::fmt;

use gpui::App;
use lib_core::{CategoryTypes, Money};
use lib_ledger::{
    AddCategory, DeleteCategory, DeleteCategoryRefusal, EditCategory, Ledger, LedgerCommand,
    MonthlyBudget, SaveCategoryRefusal,
};

use super::{dialogs::CategoriesDialog, form::CategoryForm};
use crate::{
    budgets,
    chrome::{
        dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
        dialog_host::{ConfirmDialog, DialogScope, DialogToast, Landed},
    },
};

/// What confirming a Categories Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryCommand {
    Add(AddCategory),
    Edit(EditCategory),
    Delete(DeleteCategory),
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

/// Categories 5c's write on a saved dialog: the Monthly budget text as an Onward amount from the
/// month `form` opened in, in the Personal spending Budget, or a Stop when `clears` and it is
/// blank. A locked field (a parent's rollup, an archived Budget) or text that isn't an amount
/// writes nothing.
fn monthly_budget(form: &CategoryForm, clears: bool) -> Option<MonthlyBudget> {
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
        today: form.today,
        limit,
    })
}

impl DialogForm for CategoriesDialog {
    type Command = CategoryCommand;
    type Refusal = CategoryRefusal;

    fn command(&self) -> Result<CategoryCommand, FormError> {
        match self {
            Self::Add { form, .. } if form.is_valid() => Ok(CategoryCommand::Add(AddCategory {
                name: form.name.text().trim().to_string(),
                parent_id: form.parent_id,
                category_type: form.category_type.clone().unwrap_or(CategoryTypes::Expense),
                monthly: monthly_budget(form, false),
            })),
            // `None` moves it to top level when the parent was cleared. A top-level Category's
            // chosen type cascades to its descendants only now, in the same command, so a Cancel
            // never wrote it and a refusal writes none of it.
            Self::Edit(id, form) if form.is_valid() => Ok(CategoryCommand::Edit(EditCategory {
                id: *id,
                name: form.name.text().trim().to_string(),
                parent_id: form.parent_id,
                category_type: form.category_type.clone(),
                monthly: monthly_budget(form, true),
            })),
            Self::Delete(id, form) if form.is_valid() => {
                Ok(CategoryCommand::Delete(DeleteCategory(*id)))
            }
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

impl ConfirmDialog for CategoriesDialog {
    type Outcome = CategoryOutcome;

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<CategoryOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

/// A Delete toasts the Splits it moved; a save raises none.
impl OutcomeToast for CategoryOutcome {
    fn toast(&self) -> Option<DialogToast> {
        match self {
            Self::Saved => None,
            Self::Deleted { name, moved } => Some((
                lib_toast::ToastKind::Success,
                lib_locale::msg::toast_category_deleted(name, *moved),
            )),
        }
    }
}

impl LedgerCommand for CategoryCommand {
    type Outcome = CategoryOutcome;
    type Refusal = CategoryRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<CategoryOutcome, CategoryRefusal> {
        match self {
            Self::Add(add) => {
                add.run(ledger).map_err(CategoryRefusal::Save)?;
                Ok(CategoryOutcome::Saved)
            }
            Self::Edit(edit) => {
                edit.run(ledger).map_err(CategoryRefusal::Save)?;
                Ok(CategoryOutcome::Saved)
            }
            Self::Delete(delete) => {
                let deleted = delete.run(ledger).map_err(CategoryRefusal::Delete)?;
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
    use gpui::{Entity, TestAppContext};
    use lib_core::CategoryId;
    use lib_ledger::LedgerParts;
    use lib_toast::ToastKind;

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::categories::form::DeleteCategoryForm,
    };

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                categories: lib_categories::default_categories(),
                budgets: budgets::default_budgets(
                    &lib_accounts::default_accounts(),
                    &lib_categories::default_categories(),
                    today(),
                ),
                ..LedgerParts::default()
            },
        )
    }

    fn category_count(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().categories().len())
    }

    #[gpui::test]
    fn an_invalid_form_is_not_confirmed(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = category_count(&ledger, cx);
        let mut dialog = CategoriesDialog::Add {
            parent_id: None,
            form: CategoryForm::new(today()),
        };
        assert_eq!(dialog.command(), Err(FormError));
        assert!(
            cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
                .is_none()
        );
        assert_eq!(category_count(&ledger, cx), before);
    }

    #[gpui::test]
    fn a_named_category_lands_without_a_toast(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = category_count(&ledger, cx);
        let mut form = CategoryForm::new(today());
        form.name = TextField::new("Pet care");
        let mut dialog = CategoriesDialog::Add {
            parent_id: None,
            form,
        };

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a named Category lands");

        assert_eq!(landed.outcome, CategoryOutcome::Saved);
        assert_eq!(landed.toast, None);
        assert_eq!(category_count(&ledger, cx), before + 1);
    }

    #[gpui::test]
    fn a_vanished_category_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog =
            CategoriesDialog::Delete(CategoryId::seed(9999), DeleteCategoryForm::new(""));

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert!(matches!(
            &dialog,
            CategoriesDialog::Delete(_, form)
                if form.error == Some(CategoryRefusal::Delete(DeleteCategoryRefusal::NotFound))
        ));
    }

    #[gpui::test]
    fn a_confirmed_delete_lands_and_toasts_the_moved_splits(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = category_count(&ledger, cx);
        let (id, name) = ledger.read_with(cx, |store, _| {
            let leaf = store
                .ledger()
                .categories()
                .iter()
                .find(|c| lib_categories::is_leaf(store.ledger().categories(), c.id))
                .expect("a seeded leaf Category");
            (leaf.id, leaf.name.to_string())
        });
        let name_copy = name.clone();
        let mut form = DeleteCategoryForm::new(name);
        form.confirmation_name = TextField::new(name_copy);
        let mut dialog = CategoriesDialog::Delete(id, form);

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("the seeded Category exists");

        assert!(matches!(landed.outcome, CategoryOutcome::Deleted { .. }));
        assert!(matches!(landed.toast, Some((ToastKind::Success, _))));
        assert_eq!(category_count(&ledger, cx), before - 1);
    }
}
