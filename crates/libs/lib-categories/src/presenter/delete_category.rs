//! The Delete category Dialog (`docs/ux/desktop-mockups/18-categories/README.md`'s 5d): the
//! destructive frame, the Category's transactions and Budget references quoted from its usage when
//! it opens, and the typed-back name (exact, case-sensitive) gating **Delete category**.
//!
//! The page opens it only on a leaf. Its confirm is lent the reassign port, which moves the
//! Category's Splits and Bill Plans to Uncategorised and refuses a parent or an Uncategorised still
//! in use; a refusal stays in the Dialog. It raises its own Toast on landing, so the page only
//! learns what it did.

use lib_core::CategoryId;
use lib_locale::Segment;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{count, reassign_refusal_text};
use crate::msg;
use crate::protocols::{
    CategoriesOutcome, CategoriesRouter, CategoryChange, CategoryReassign, CategoryUsage,
    CategoryUse, SharedCategories,
};

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
    id: CategoryId,
    name: String,
    uses: CategoryUse,
    confirm_text: String,
    refusal: Option<String>,
}

impl DeleteCategoryPresenter {
    /// The Dialog for Category `id`, its use read from `usage`, or `None` when it is gone.
    pub fn new(
        categories: &SharedCategories,
        usage: &dyn CategoryUsage,
        id: CategoryId,
    ) -> Option<Self> {
        let category = categories.borrow().category(id)?;
        Some(Self {
            id,
            name: category.name,
            uses: usage.category_usage(id),
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: DeleteCategoryEvent, router: &mut dyn CategoriesRouter) {
        match event {
            DeleteCategoryEvent::ConfirmText(text) => self.confirm_text = text,
            DeleteCategoryEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Deletes the Category through `reassign` once its name is typed back, raises the Toast and
    /// closes the Dialog; a refusal stays in the Dialog.
    pub fn confirm(
        &mut self,
        reassign: &mut dyn CategoryReassign,
        router: &mut dyn CategoriesRouter,
    ) {
        if !self.can_confirm() {
            return;
        }
        match reassign.delete_category(self.id) {
            Ok(deleted) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_category_deleted(
                        &deleted.name,
                        count(deleted.splits_moved),
                    ),
                ));
                router.close_dialog(Some(CategoriesOutcome {
                    change: CategoryChange::Deleted,
                    id: self.id,
                    name: deleted.name,
                }));
            }
            Err(refusal) => self.refusal = Some(reassign_refusal_text(refusal)),
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
            warning: msg::categories_delete_warning(count(self.uses.splits)),
            reference: msg::categories_delete_reference(count(self.uses.budgets)),
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
    use super::super::test_support::stub;
    use super::*;
    use crate::CategoryRepository as _;
    use crate::fake::{InMemoryCategoryUsage, InMemoryReassign, RecordingRouter, RouterCall};
    use crate::protocols::ReassignRefusal;

    const RENT: CategoryId = CategoryId::seed(2);

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
        let (_, shared) = stub();
        let mut usage = InMemoryCategoryUsage::default();
        usage.uses.insert(
            RENT,
            CategoryUse {
                splits: 3,
                budgets: 1,
            },
        );
        let view = DeleteCategoryPresenter::new(&shared, &usage, RENT)
            .expect("seeded")
            .view_model();

        assert_eq!(view.frame.title, "Delete category \u{2014} Rent");
        assert!(view.frame.destructive);
        assert_eq!(
            plain(&view.warning),
            "This category has 3 transactions. Deleting it cannot be undone."
        );
        assert_eq!(
            view.reference,
            "Splits will be moved to Uncategorised \u{b7} 1 budget attached"
        );
        assert!(DeleteCategoryPresenter::new(&shared, &usage, CategoryId::new()).is_none());
    }

    #[test]
    fn confirm_waits_for_the_exact_name_then_deletes_and_raises_its_toast() {
        let (categories, shared) = stub();
        let mut reassign = InMemoryReassign::new(categories.clone());
        reassign.splits_moved = 2;
        let mut dialog =
            DeleteCategoryPresenter::new(&shared, &InMemoryCategoryUsage::default(), RENT)
                .expect("seeded");
        let mut router = RecordingRouter::default();

        typed(&mut dialog, "rent");
        dialog.confirm(&mut reassign, &mut router);
        assert_eq!(router.take(), []);
        assert!(categories.borrow().category(RENT).is_some());

        typed(&mut dialog, "Rent");
        dialog.confirm(&mut reassign, &mut router);

        assert_eq!(
            router.take(),
            [
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Deleted category Rent \u{b7} 2 splits moved to Uncategorised".into()
                )),
                RouterCall::CloseDialog(Some(CategoriesOutcome {
                    change: CategoryChange::Deleted,
                    id: RENT,
                    name: "Rent".into(),
                })),
            ]
        );
        assert!(categories.borrow().category(RENT).is_none());
    }

    #[test]
    fn a_parent_or_an_uncategorised_in_use_is_refused_in_the_dialog() {
        let (categories, shared) = stub();
        let usage = InMemoryCategoryUsage::default();
        let mut reassign = InMemoryReassign::new(categories.clone());
        let mut router = RecordingRouter::default();
        let mut dialog = DeleteCategoryPresenter::new(&shared, &usage, RENT).expect("seeded");
        typed(&mut dialog, "Rent");

        // A child added under Rent while the Dialog was open makes it a parent.
        categories.borrow_mut().upsert_category(crate::Category {
            id: CategoryId::seed(50),
            name: "Garage".into(),
            parent: Some(RENT),
            category_type: lib_core::CategoryTypes::Expense,
        });
        dialog.confirm(&mut reassign, &mut router);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("Delete or move its subcategories first.")
        );

        reassign.refusal = Some(ReassignRefusal::UncategorisedInUse);
        dialog.confirm(&mut reassign, &mut router);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
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
}
