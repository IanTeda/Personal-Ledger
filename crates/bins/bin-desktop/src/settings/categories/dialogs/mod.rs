//! The Categories page's Dialogs (ADR-0037's amendment): Add, Edit and Delete category. [`CategoriesDialog`]
//! is the value the Dialog host holds while one is open (`OpenDialog::Categories`). It draws
//! itself from the Ledger's tree (`Dialog::render`), each click editing it through the host's one
//! edit callback ([`CategoriesDialogHandlers`]), and reads itself back for the UI tests.

pub mod add;
pub mod delete;
pub mod edit;

use std::rc::Rc;

use gpui::{AnyElement, App, Entity, div, prelude::*};
use lib_budgets::Budgets;
use lib_categories::{Category, depth, descendants_inclusive, is_leaf, path};
use lib_core::{CategoryId, CategoryTypes};
use lib_transactions::Transaction;

use super::{
    apply::CategoryRefusal,
    form::{CategoryForm, DeleteCategoryForm},
};
use crate::{
    chrome::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
    dialog,
    form::field::TextField,
    ledger_store::LedgerStore,
};

/// The handlers a Categories Dialog binds: the Add and Edit forms' field, parent and type changes,
/// and cancel and confirm, which the Delete confirm shares.
pub struct CategoriesDialogHandlers {
    pub on_field_click: add::OnFieldClick,
    pub on_parent_change: add::OnParentChange,
    pub on_type_change: add::OnTypeChange,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

impl CategoriesDialogHandlers {
    /// Each click edits the open Categories Dialog through `actions`; a parent or type change
    /// reads the tree from `ledger`.
    fn new(ledger: &Entity<LedgerStore>, actions: &DialogActions) -> Self {
        let on_field_click: add::OnFieldClick = {
            let actions = actions.clone();
            Rc::new(move |field, _window, cx| {
                actions.edit(cx, |dialog: &mut CategoriesDialog, _| {
                    if let Some(form) = dialog.form_mut() {
                        form.focus_field(field);
                    }
                    None
                });
            })
        };
        let on_parent_change: add::OnParentChange = {
            let (actions, ledger) = (actions.clone(), ledger.clone());
            Rc::new(move |parent_id, _window, cx| {
                actions.edit(cx, |dialog: &mut CategoriesDialog, cx| {
                    dialog.change_parent(parent_id, ledger.read(cx).ledger().categories());
                    None
                });
            })
        };
        let on_type_change: add::OnTypeChange = {
            let (actions, ledger) = (actions.clone(), ledger.clone());
            Rc::new(move |category_type, _window, cx| {
                actions.edit(cx, |dialog: &mut CategoriesDialog, cx| {
                    dialog.change_type(category_type, ledger.read(cx).ledger().categories());
                    None
                });
            })
        };
        Self {
            on_field_click,
            on_parent_change,
            on_type_change,
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        }
    }
}

/// A refusal the domain made on confirm, as the dialogs' last line.
pub(crate) fn refusal_line(refusal: &CategoryRefusal, cx: &App) -> AnyElement {
    div()
        .text_size(gpui::px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(crate::theme::color::accent_text(cx))
        .child(refusal.to_string())
        .into_any_element()
}

/// The categories dialog's state: Add { parent } / Edit(id) / Delete(id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoriesDialog {
    /// Adding a new category under a parent (or None for top-level).
    Add {
        parent_id: Option<CategoryId>,
        form: CategoryForm,
    },
    /// Editing the category with this [`Category::id`].
    Edit(CategoryId, CategoryForm),
    /// Deleting the category with this [`Category::id`], once its name has been typed back.
    Delete(CategoryId, DeleteCategoryForm),
}

impl CategoriesDialog {
    /// The form behind the Add and Edit dialogs; Delete has its own, single-field form.
    pub fn form(&self) -> Option<&CategoryForm> {
        match self {
            CategoriesDialog::Add { form, .. } | CategoriesDialog::Edit(_, form) => Some(form),
            CategoriesDialog::Delete(_, _) => None,
        }
    }

    /// The mutable form behind the Add and Edit dialogs; Delete has its own, single-field form.
    pub fn form_mut(&mut self) -> Option<&mut CategoryForm> {
        match self {
            CategoriesDialog::Add { form, .. } | CategoriesDialog::Edit(_, form) => Some(form),
            CategoriesDialog::Delete(_, _) => None,
        }
    }

    /// Picks the Add or Edit form's parent; a child takes its parent's type.
    fn change_parent(&mut self, parent_id: Option<CategoryId>, categories: &[Category]) {
        let parent_type = parent_id.and_then(|parent_id| {
            categories
                .iter()
                .find(|c| c.id == parent_id)
                .map(|parent| parent.category_type.clone())
        });
        if let Some(form) = self.form_mut() {
            form.parent_id = parent_id;
            if parent_type.is_some() {
                form.category_type = parent_type;
            }
        }
    }

    /// Picks a top-level Category's type; a child's is its parent's, so it stays. The choice is
    /// held on the form and cascaded to descendants on confirm, so Cancel reverts it.
    fn change_type(&mut self, category_type: CategoryTypes, categories: &[Category]) {
        let can_change_type = match self {
            Self::Add { form, .. } => form.parent_id.is_none(),
            Self::Edit(id, _) => categories
                .iter()
                .find(|c| c.id == *id)
                .is_some_and(|c| c.parent.is_none()),
            Self::Delete(..) => false,
        };
        if can_change_type && let Some(form) = self.form_mut() {
            form.category_type = Some(category_type);
        }
    }
}

impl Dialog for CategoriesDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.form_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add { form, .. } | Self::Edit(_, form) => form.focused_text(),
            Self::Delete(_, form) => Some(&mut form.confirmation_name),
        }
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.form_mut() {
            form.focused = form.focused.next();
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add { form, .. } | Self::Edit(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let ledger = scope.ledger.read(cx).ledger();
        Some(self.draw(
            ledger.categories(),
            ledger.transactions(),
            ledger.budgets(),
            CategoriesDialogHandlers::new(&scope.ledger, actions),
            cx,
        ))
    }

    /// Add and Edit read back their Name, and an Add under a parent that parent's name; Delete
    /// reads back its typed-back name.
    fn readback(&self, scope: &DialogScope, cx: &App) -> Option<DialogReadback> {
        let (kind, values) = match self {
            Self::Add { parent_id, form } => {
                let categories = scope.ledger.read(cx).ledger().categories();
                let parent = parent_id
                    .and_then(|id| categories.iter().find(|c| c.id == id))
                    .map(|parent| ("parent", parent.name.clone()));
                let name = ("name", form.name.text().to_string());
                ("AddCategory", std::iter::once(name).chain(parent).collect())
            }
            Self::Edit(_, form) => (
                "EditCategory",
                vec![
                    ("name", form.name.text().to_string()),
                    ("type", format!("{:?}", form.category_type)),
                ],
            ),
            Self::Delete(_, form) => (
                "DeleteCategory",
                vec![("confirm", form.confirmation_name.text().to_string())],
            ),
        };
        Some(DialogReadback { kind, values })
    }
}

impl CategoriesDialog {
    /// Draws the open Dialog. Its counts and parent choices come from the Ledger's `categories`,
    /// `transactions` and `budgets`; Delete draws nothing if its Category has gone.
    fn draw(
        &self,
        categories: &[Category],
        transactions: &[Transaction],
        budgets: &Budgets,
        handlers: CategoriesDialogHandlers,
        cx: &App,
    ) -> AnyElement {
        match self {
            Self::Add { form, .. } => {
                let parent_options: Vec<_> = categories
                    .iter()
                    .map(|c| add::ParentOption {
                        id: Some(c.id),
                        label: path(categories, c.id).unwrap_or_else(|| c.name.clone()),
                        // Can't add children to depth-2 categories
                        is_available: depth(categories, c.id) < 2,
                    })
                    .collect();
                add::render(form, &parent_options, categories, handlers, cx)
            }
            Self::Edit(category_id, form) => {
                let descendants = categories
                    .iter()
                    .find(|c| c.id == *category_id)
                    .map(|_| descendants_inclusive(categories, *category_id))
                    .unwrap_or_default();
                let parent_options: Vec<_> = categories
                    .iter()
                    // Exclude the category itself and its descendants (to prevent cycles)
                    .filter(|c| c.id != *category_id && !descendants.contains(&c.id))
                    // Check depth: can't be at depth 2 or deeper
                    .filter(|c| depth(categories, c.id) < 2)
                    .map(|c| edit::ParentOption {
                        id: Some(c.id),
                        label: path(categories, c.id).unwrap_or_else(|| c.name.clone()),
                        is_available: true,
                    })
                    .collect();
                edit::render(
                    *category_id,
                    form,
                    &parent_options,
                    categories,
                    split_count(categories, transactions, *category_id),
                    !is_leaf(categories, *category_id),
                    handlers,
                    cx,
                )
            }
            Self::Delete(category_id, form) => {
                let Some(category) = categories.iter().find(|c| c.id == *category_id) else {
                    return div().into_any_element();
                };
                delete::render(
                    category,
                    form,
                    split_count(categories, transactions, *category_id),
                    // Count budgets attached to this category
                    budgets.limit_count(*category_id),
                    handlers.on_cancel,
                    handlers.on_confirm,
                    cx,
                )
            }
        }
    }
}

/// The Splits in `category_id` and, for a parent, in all its descendants.
fn split_count(
    categories: &[Category],
    transactions: &[Transaction],
    category_id: CategoryId,
) -> usize {
    descendants_inclusive(categories, category_id)
        .iter()
        .flat_map(|cat_id| {
            transactions
                .iter()
                .flat_map(move |t| t.splits.iter().filter(move |s| s.category_id == *cat_id))
        })
        .count()
}
