//! The Categories dialogs' form state: the Add, Edit and Delete category forms and the dialog
//! they sit behind. `gpui`-free, like the rest of this domain; `view::categories` renders it.

use lib_core::CategoryTypes;

use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
};

/// The Add/Edit category dialog's fields, in `Tab` order: Name, Parent, Type, Budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CategoryField {
    #[default]
    Name,
    Parent,
    Type,
    Budget,
}

impl CategoryField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Parent,
            Self::Parent => Self::Type,
            Self::Type => Self::Budget,
            Self::Budget => Self::Name,
        }
    }
}

/// Why the Monthly budget field can't be typed into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetLock {
    /// A parent only rolls up its children: the field shows their sum.
    Parent,
    /// The Budget the field reads and writes (named here) is archived, and so read-only.
    Archived(String),
}

/// A form for adding or editing a category: name, parent, type, and monthly budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryForm {
    pub name: TextField,
    pub parent_id: Option<u32>,
    pub category_type: Option<CategoryTypes>,
    pub budget: TextField,
    /// `Some` makes the Monthly budget field read-only; saving then leaves the Budget alone.
    pub budget_lock: Option<BudgetLock>,
    pub focused: CategoryField,
}

impl Default for CategoryForm {
    fn default() -> Self {
        CategoryForm {
            name: TextField::default(),
            parent_id: None,
            category_type: Some(CategoryTypes::Expense),
            budget: TextField::default(),
            budget_lock: None,
            focused: CategoryField::Name,
        }
    }
}

impl CategoryForm {
    pub fn focus_field(&mut self, field: CategoryField) {
        self.focused = field;
    }

    /// Only a blank name blocks saving; a duplicate sibling name is the view's warning.
    pub fn is_valid(&self) -> bool {
        !self.name.is_blank()
    }

    /// The text field typing edits: Name, or Monthly budget unless it is locked.
    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            CategoryField::Name => Some(&mut self.name),
            CategoryField::Budget if self.budget_lock.is_none() => Some(&mut self.budget),
            CategoryField::Budget | CategoryField::Parent | CategoryField::Type => None,
        }
    }

    /// Typing and `Backspace` on a field with nothing to type into (the two selects, a locked
    /// Monthly budget) are swallowed so they never fall through to the shell, and the Monthly
    /// budget only takes digits and `.`.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Char(ch) => {
                let on_budget = self.focused == CategoryField::Budget;
                match self.focused_text() {
                    None => {}
                    Some(field) if on_budget => {
                        if ch.is_numeric() || ch == '.' {
                            field.push(ch);
                        }
                    }
                    Some(_) => return None,
                }
            }
            DialogKey::Backspace if self.focused_text().is_none() => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }
}

/// A form for deleting a category: confirmation name. The category's name is copied in at open
/// so the form validates without `Shell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCategoryForm {
    pub confirmation_name: TextField,
    name: String,
}

impl DeleteCategoryForm {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            confirmation_name: TextField::default(),
            name: name.into(),
        }
    }

    /// A plain case-sensitive `==`, so it can't be confirmed by habit.
    pub fn is_valid(&self) -> bool {
        self.confirmation_name.text() == self.name
    }
}

/// The categories dialog's state: Add { parent } / Edit(id) / Delete(id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoriesDialog {
    /// Adding a new category under a parent (or None for top-level).
    Add {
        parent_id: Option<u32>,
        form: CategoryForm,
    },
    /// Editing the category with this [`Category::id`].
    Edit(u32, CategoryForm),
    /// Deleting the category with this [`Category::id`], once its name has been typed back.
    Delete(u32, DeleteCategoryForm),
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
}
