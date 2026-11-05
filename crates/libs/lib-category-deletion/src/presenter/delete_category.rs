//! The Delete category Dialog (`docs/ux/desktop-mockups/18-categories/README.md`'s 5d): the
//! destructive frame, the Category's transactions and Budget references quoted from its use when
//! it opens, and the typed-back name (exact, case-sensitive) gating **Delete category**.
//!
//! The page opens it only on a leaf. Its confirm runs [`DeleteCategory`] through the deletion
//! port, which moves the Category's Splits and Bill Plans to Uncategorised and refuses a parent or
//! an Uncategorised still in use, in the one commit of the input; a refusal stays in the Dialog.
//! It raises its own Toast on landing, so the page only learns what it did.

use lib_categories::{CategoriesOutcome, CategoryChange};
use lib_core::CategoryId;
use lib_locale::Segment;
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{DialogFrame, TextFieldView, count};

use super::refusal_text;
use crate::interactor::DeleteCategory;
use crate::msg;
use crate::protocols::{CategoryDeletionReads, SharedCategoryDeletion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteCategoryEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCategoryViewModel {
    pub frame: DialogFrame,
    /// How many transactions it has, tagged `strong`.
    pub warning: Vec<Segment>,
    /// Where its Splits go and the Budgets it is attached to.
    pub reference: String,
    /// `Type Rent to confirm`.
    pub confirm: TextFieldView,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct DeleteCategoryPresenter {
    deletion: SharedCategoryDeletion,
    id: CategoryId,
    name: String,
    /// The Splits naming the Category, read when the Dialog opens.
    splits: usize,
    /// The Budgets holding a Limit for it, read when the Dialog opens.
    budgets: usize,
    confirm_text: String,
    refusal: Option<String>,
}

impl DeleteCategoryPresenter {
    /// The Dialog for Category `id`, its use read from `reads` when it opens, or `None` when it is
    /// gone.
    pub fn new(
        deletion: SharedCategoryDeletion,
        reads: &dyn CategoryDeletionReads,
        id: CategoryId,
    ) -> Option<Self> {
        let in_use = reads.category_in_use(id)?;
        Some(Self {
            deletion,
            id,
            name: in_use.category.name,
            splits: in_use.splits,
            budgets: in_use.budgets,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(
        &mut self,
        event: DeleteCategoryEvent,
        router: &mut dyn DialogRouter<CategoriesOutcome>,
    ) {
        match event {
            DeleteCategoryEvent::ConfirmText(text) => self.confirm_text = text,
            DeleteCategoryEvent::Cancel => router.close(None),
        }
    }

    /// Reads a Dialog command or typed key: `select` and `submit` delete once the name is typed
    /// back, and typing goes to the typed-back name, the only field.
    pub fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<CategoriesOutcome>) {
        match input {
            DialogInput::Select | DialogInput::Submit => self.confirm(router),
            DialogInput::Char(ch) => self.confirm_text.push(ch),
            DialogInput::Backspace => {
                self.confirm_text.pop();
            }
            _ => {}
        }
    }

    /// Deletes the Category once its name is typed back, raises the Toast and closes the Dialog;
    /// a refusal stays in the Dialog.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<CategoriesOutcome>) {
        if !self.can_confirm() {
            return;
        }
        let outcome = DeleteCategory { id: self.id }.run(&mut *self.deletion.borrow_mut());
        match outcome {
            Ok(deleted) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_category_deleted(
                        &deleted.name,
                        count(deleted.splits_moved),
                    ),
                ));
                router.close(Some(CategoriesOutcome {
                    change: CategoryChange::Deleted,
                    id: deleted.id,
                    name: deleted.name,
                }));
            }
            Err(refusal) => self.refusal = Some(refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> DeleteCategoryViewModel {
        DeleteCategoryViewModel {
            frame: DialogFrame {
                title: msg::categories_delete_title(&self.name),
                submit: msg::categories_delete_submit(),
                can_confirm: self.can_confirm(),
                destructive: true,
                hints: Vec::new(),
            },
            warning: msg::categories_delete_warning(count(self.splits)),
            reference: msg::categories_delete_reference(count(self.budgets)),
            confirm: TextFieldView {
                label: msg::categories_delete_confirm_label(&self.name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            },
            refusal: self.refusal.clone(),
        }
    }

    /// The typed name must match exactly, case and all.
    fn can_confirm(&self) -> bool {
        self.confirm_text == self.name
    }
}

#[cfg(test)]
mod tests {
    use lib_categories::CategoryRepository as _;
    use lib_categories::fake::{RecordingRouter, RouterCall};

    use super::super::test_support::shared_world;
    use super::*;
    use crate::fake::world::{GROCERIES, RENT, UNCATEGORISED_INCOME};
    use crate::protocols::CategoryDeletion as _;

    fn typed(dialog: &mut DeleteCategoryPresenter, text: &str) {
        dialog.handle(
            DeleteCategoryEvent::ConfirmText(text.to_string()),
            &mut RecordingRouter::default(),
        );
    }

    fn plain(segments: &[Segment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn it_quotes_the_categorys_transactions_and_budgets() {
        let (rows, deletion) = shared_world();
        let view = DeleteCategoryPresenter::new(deletion.clone(), &*rows.borrow(), GROCERIES)
            .expect("a world Category")
            .view_model();

        assert_eq!(view.frame.title, "Delete category \u{2014} Groceries");
        assert_eq!(view.frame.submit, "Delete category");
        assert!(view.frame.destructive);
        assert!(!view.frame.can_confirm);
        assert_eq!(
            plain(&view.warning),
            "This category has 2 transactions. Deleting it cannot be undone."
        );
        assert_eq!(
            view.reference,
            "Splits will be moved to Uncategorised \u{b7} 2 budgets attached"
        );
        assert_eq!(view.confirm.label, "Type Groceries to confirm");
        assert!(
            DeleteCategoryPresenter::new(deletion, &*rows.borrow(), CategoryId::seed(999))
                .is_none()
        );
    }

    #[test]
    fn confirm_waits_for_the_exact_name_then_deletes_and_raises_its_toast() {
        let (rows, deletion) = shared_world();
        let mut dialog = DeleteCategoryPresenter::new(deletion, &*rows.borrow(), GROCERIES)
            .expect("a world Category");
        let mut router = RecordingRouter::default();

        typed(&mut dialog, "groceries");
        dialog.confirm(&mut router);
        assert_eq!(router.take(), []);
        assert!(rows.borrow().categories.category(GROCERIES).is_some());

        typed(&mut dialog, "Groceries");
        dialog.confirm(&mut router);

        assert_eq!(
            router.take(),
            [
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Deleted category Groceries \u{b7} 2 splits moved to Uncategorised".into()
                )),
                RouterCall::CloseDialog(Some(CategoriesOutcome {
                    change: CategoryChange::Deleted,
                    id: GROCERIES,
                    name: "Groceries".into(),
                })),
            ]
        );
        assert!(rows.borrow().categories.category(GROCERIES).is_none());
        assert!(
            rows.borrow().transactions_using(GROCERIES).is_empty(),
            "its Splits moved"
        );
    }

    #[test]
    fn typing_builds_the_name_and_select_deletes_only_once_it_is_exact() {
        let (rows, deletion) = shared_world();
        let mut dialog = DeleteCategoryPresenter::new(deletion, &*rows.borrow(), RENT)
            .expect("a world Category");
        let mut router = RecordingRouter::default();
        let mut press = |dialog: &mut DeleteCategoryPresenter, input| {
            dialog.input(input, &mut router);
        };

        for ch in "Renx".chars() {
            press(&mut dialog, DialogInput::Char(ch));
        }
        press(&mut dialog, DialogInput::Select);
        press(&mut dialog, DialogInput::NextField);
        assert!(
            rows.borrow().categories.category(RENT).is_some(),
            "not the name yet"
        );
        press(&mut dialog, DialogInput::Backspace);
        press(&mut dialog, DialogInput::Char('t'));
        assert_eq!(dialog.view_model().confirm.text, "Rent");
        press(&mut dialog, DialogInput::Submit);

        assert!(rows.borrow().categories.category(RENT).is_none());
        assert!(matches!(
            router.take().last(),
            Some(RouterCall::CloseDialog(Some(_)))
        ));
    }

    #[test]
    fn a_parent_or_an_uncategorised_in_use_is_refused_in_the_dialog() {
        let (rows, deletion) = shared_world();
        let mut router = RecordingRouter::default();
        let mut dialog = DeleteCategoryPresenter::new(deletion.clone(), &*rows.borrow(), RENT)
            .expect("a world Category");
        typed(&mut dialog, "Rent");

        // A child added under Rent while the Dialog was open makes it a parent.
        rows.borrow_mut()
            .categories
            .upsert_category(lib_categories::Category {
                id: CategoryId::seed(50),
                name: "Garage".into(),
                parent: Some(RENT),
                category_type: lib_core::CategoryTypes::Expense,
            });
        dialog.confirm(&mut router);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("Delete or move its subcategories first.")
        );

        let mut uncategorised =
            DeleteCategoryPresenter::new(deletion, &*rows.borrow(), UNCATEGORISED_INCOME)
                .expect("a world Category");
        typed(&mut uncategorised, "Uncategorised");
        uncategorised.confirm(&mut router);
        assert_eq!(
            uncategorised.view_model().refusal.as_deref(),
            Some("Uncategorised can't be deleted while transactions or bills use it.")
        );
        assert_eq!(router.take(), [], "a refusal keeps the Dialog open");
        assert_eq!(
            {
                dialog.handle(DeleteCategoryEvent::Cancel, &mut router);
                router.take()
            },
            [RouterCall::CloseDialog(None)]
        );
    }

    #[test]
    fn a_category_gone_under_the_dialog_is_refused() {
        let (rows, deletion) = shared_world();
        let mut dialog = DeleteCategoryPresenter::new(deletion, &*rows.borrow(), RENT)
            .expect("a world Category");
        rows.borrow_mut()
            .categories
            .categories
            .retain(|category| category.id != RENT);
        typed(&mut dialog, "Rent");

        dialog.confirm(&mut RecordingRouter::default());

        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This category has since been deleted.")
        );
    }
}
