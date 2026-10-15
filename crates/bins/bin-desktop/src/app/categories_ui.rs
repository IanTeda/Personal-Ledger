//! The Categories destination's wiring: the Settings Categories tree's keys, clicks and folds, and the Add, Edit and Delete dialogs. The pure tree rules live in `lib_categories`, the chrome in `settings::categories` and `view::categories`, and the tree's rows and cursor are the `CategoriesStore` and `CategoriesView` Entities (ADR-0032).

use super::Shell;
use crate::categories::{
    apply::{CategoriesApply, CategoryOutcome},
    form::CategoriesDialog,
};

use crate::{
    budgets, categories,
    chrome::{dialog_apply, dialog_host::OpenDialog},
    form::field::TextField,
    navigation::{
        key_router::Movement,
        nav::{FocusZone, Noun},
    },
    settings::{SettingsFocus, SettingsPage},
};

use gpui::{App, Context, Keystroke};
use lib_core::CategoryTypes;
use lib_toast::ToastKind;

impl Shell {
    /// Whether Settings' Categories tree owns the keyboard: the page, not the index, has focus.
    pub(super) fn settings_categories_page_has_focus(&self, cx: &App) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.views.settings.read(cx).focus() == SettingsFocus::Page
            && self.views.settings.read(cx).selected_section() == SettingsPage::Categories
    }

    /// Whether `left` has something to do inside the Categories tree: fold an open parent, or
    /// climb from a nested row to its parent.
    pub(super) fn settings_categories_left_is_local(&self, cx: &App) -> bool {
        let view = self.categories_view.read(cx);
        let Some(id) = view.selected_id() else {
            return false;
        };
        let categories = self.categories(cx);
        let open_parent = view.expanded().contains(&id) && !categories::is_leaf(categories, id);
        open_parent
            || categories
                .iter()
                .any(|category| category.id == id && category.parent.is_some())
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Categories tree's visible rows, Expense then
    /// Income. With no row selected (or the selected one folded away) the first press lands on
    /// the first row.
    pub(super) fn apply_settings_categories_movement(&mut self, movement: Movement, cx: &mut App) {
        let Some(step) = movement.step() else {
            return;
        };
        let ids: Vec<u32> = categories::settings_rows(
            self.categories(cx),
            self.categories_view.read(cx).expanded(),
        )
        .iter()
        .map(|row| row.id)
        .collect();
        if ids.is_empty() {
            return;
        }
        let current = self.categories_view.read(cx).selected_id();
        self.select_category(lib_list_cursor::step_id(&ids, current, step), cx);
    }

    /// Moves the Settings Categories cursor to `id`, held in the view Entity.
    pub(super) fn select_category(&self, id: Option<u32>, cx: &mut App) {
        self.categories_view
            .update(cx, |view, cx| view.set_selected_id(id, cx));
    }

    /// Edits the Settings Categories tree's expanded list through its view Entity.
    pub(super) fn update_expanded_categories(
        &self,
        cx: &mut App,
        edit: impl FnOnce(&mut Vec<u32>),
    ) {
        self.categories_view
            .update(cx, |view, cx| view.update_expanded(cx, edit));
    }

    /// `right`: opens a folded parent, or steps into an open one's first child. `left`: folds an
    /// open parent, or climbs to the parent of a nested row.
    pub(super) fn step_settings_categories_fold(&mut self, forward: bool, cx: &mut App) {
        let Some(id) = self.categories_view.read(cx).selected_id() else {
            return;
        };
        let is_parent = !categories::is_leaf(self.categories(cx), id);
        let open = self.categories_view.read(cx).expanded().contains(&id);
        if forward {
            if !is_parent {
                return;
            }
            if open {
                self.apply_settings_categories_movement(Movement::Next, cx);
            } else {
                self.update_expanded_categories(cx, |expanded| expanded.push(id));
            }
        } else if is_parent && open {
            self.update_expanded_categories(cx, |expanded| expanded.retain(|&other| other != id));
        } else if let Some(parent) = self
            .categories(cx)
            .iter()
            .find(|category| category.id == id)
            .and_then(|category| category.parent)
        {
            self.select_category(Some(parent), cx);
        }
    }

    /// Keyboard input while on the Categories page: `n` adds a top-level category, `N` (shift+n)
    /// adds a sub-category to the selected one, `e` edits the selected category, `d` deletes it.
    pub(super) fn handle_categories_key(&mut self, keystroke: &Keystroke, cx: &mut App) -> bool {
        let on_settings_page = self.settings_categories_page_has_focus(cx);
        if !on_settings_page {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        let shift = modifiers.shift;
        let has_mod = modifiers.control || modifiers.alt || modifiers.platform;
        if has_mod {
            return false;
        }

        // The id, not a borrow of the tree: the arms below open dialogs, which need `cx` mutably.
        let selected_id = self.categories_view.read(cx).selected_id().and_then(|id| {
            self.categories(cx)
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.id)
        });

        match keystroke.key.as_str() {
            "n" => {
                if shift {
                    if let Some(id) = selected_id {
                        self.open_add_categories_dialog(Some(id), cx);
                    }
                } else {
                    self.open_add_categories_dialog(None, cx);
                }
                true
            }
            "e" => {
                if let Some(id) = selected_id {
                    self.open_edit_categories_dialog(id, cx);
                }
                true
            }
            "d" => {
                if let Some(id) = selected_id {
                    if !categories::is_leaf(self.categories(cx), id) {
                        self.chrome.status_message =
                            Some(crate::msg::desktop_status_delete_children_first());
                    } else {
                        self.open_delete_categories_dialog(id, cx);
                    }
                }
                true
            }
            "right" | "left" if on_settings_page => {
                self.step_settings_categories_fold(keystroke.key == "right", cx);
                true
            }
            _ => false,
        }
    }

    /// Why the Categories dialogs' Monthly budget field is read-only for `category_id` (`None`
    /// for a Category being added): the Personal spending Budget it reads and writes is archived,
    /// or the Category is a parent and so only rolls up.
    pub(super) fn categories_budget_lock(
        &self,
        category_id: Option<u32>,
        cx: &App,
    ) -> Option<categories::form::BudgetLock> {
        if category_id.is_some_and(|id| !categories::is_leaf(self.categories(cx), id)) {
            return Some(categories::form::BudgetLock::Parent);
        }
        self.budgets(cx)
            .get(budgets::PERSONAL_SPENDING_ID)
            .filter(|budget| budget.is_archived())
            .map(|budget| categories::form::BudgetLock::Archived(budget.name.clone()))
    }

    /// Opens the Add categories dialog pre-scoped to parent_id (None for top-level).
    pub(super) fn open_add_categories_dialog(&mut self, parent_id: Option<u32>, cx: &App) {
        // A child takes its parent's type, locked in the form; a top-level one starts as Expense.
        let category_type = match parent_id {
            Some(id) => self
                .categories(cx)
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.category_type.clone()),
            None => Some(CategoryTypes::Expense),
        };
        let form = categories::form::CategoryForm {
            name: TextField::default(),
            parent_id,
            category_type,
            budget: TextField::default(),
            budget_lock: self.categories_budget_lock(None, cx),
            focused: categories::form::CategoryField::Name,
            error: None,
        };
        self.open_dialog(OpenDialog::Categories(CategoriesDialog::Add {
            parent_id,
            form,
        }));
    }

    /// Opens the Edit categories dialog on `id`, pre-filled. Monthly budget shows the current
    /// month's amount in the Personal spending Budget (a parent's is its children's sum).
    pub(super) fn open_edit_categories_dialog(&mut self, category_id: u32, cx: &App) {
        if let Some(category) = self.categories(cx).iter().find(|c| c.id == category_id) {
            let budget_str = self
                .budgets(cx)
                .monthly_limit(
                    budgets::PERSONAL_SPENDING_ID,
                    self.categories(cx),
                    category_id,
                    self.today,
                )
                .map(|amount| amount.0.to_string())
                .unwrap_or_default();
            let form = categories::form::CategoryForm {
                name: TextField::new(category.name.as_str()),
                parent_id: category.parent,
                category_type: Some(category.category_type.clone()),
                budget: TextField::new(budget_str),
                budget_lock: self.categories_budget_lock(Some(category_id), cx),
                focused: categories::form::CategoryField::Name,
                error: None,
            };
            self.open_dialog(OpenDialog::Categories(CategoriesDialog::Edit(
                category_id,
                form,
            )));
        }
    }

    /// Raises the error Toast for a Categories write the store or Budgets refused, so the dialog
    /// closes with the reason on screen rather than dropping the write silently.
    fn raise_category_save_failed(&mut self, reason: &str) {
        self.raise_toast(
            ToastKind::Error,
            lib_locale::msg::toast_save_failed(&lib_locale::msg::toast_entity_category(), reason),
        );
    }

    /// Raises the Toast for a refused Categories store write, if it was refused.
    fn report_category_write(&mut self, result: Result<(), categories::CategoryError>) {
        if let Err(error) = result {
            self.raise_category_save_failed(&error.to_string());
        }
    }

    pub(super) fn handle_categories_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_categories_dialog(None, cx);
        cx.notify();
    }

    pub(super) fn handle_categories_add_sub_click(
        &mut self,
        parent_id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.select_category(Some(parent_id), cx);
        self.open_add_categories_dialog(Some(parent_id), cx);
        cx.notify();
    }

    pub(super) fn handle_categories_edit_click(
        &mut self,
        category_id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        if self.categories(cx).iter().any(|c| c.id == category_id) {
            self.select_category(Some(category_id), cx);
            self.open_edit_categories_dialog(category_id, cx);
            cx.notify();
        }
    }

    pub(super) fn handle_categories_delete_click(
        &mut self,
        category_id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        if !categories::is_leaf(self.categories(cx), category_id) {
            self.chrome.status_message = Some(crate::msg::desktop_status_delete_children_first());
            cx.notify();
            return;
        }

        self.open_delete_categories_dialog(category_id, cx);
        cx.notify();
    }

    /// Opens the Delete category dialog on `category_id`, copying its name in so the form
    /// validates without `Shell`. A no-op if the category is gone.
    pub(super) fn open_delete_categories_dialog(&mut self, category_id: u32, cx: &App) {
        let Some(category) = self.categories(cx).iter().find(|c| c.id == category_id) else {
            return;
        };
        let form = categories::form::DeleteCategoryForm::new(category.name.as_str());
        self.open_dialog(OpenDialog::Categories(CategoriesDialog::Delete(
            category_id,
            form,
        )));
    }

    pub(super) fn handle_categories_dialog_field_click(
        &mut self,
        field: categories::form::CategoryField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.focus_field(field);
            cx.notify();
        }
    }

    pub(super) fn handle_categories_dialog_parent_change(
        &mut self,
        parent_id: Option<u32>,
        cx: &mut Context<'_, Self>,
    ) {
        // A child takes its parent's type.
        let parent_type = parent_id.and_then(|parent_id| {
            self.categories(cx)
                .iter()
                .find(|c| c.id == parent_id)
                .map(|parent| parent.category_type.clone())
        });
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.parent_id = parent_id;
            if parent_type.is_some() {
                form.category_type = parent_type;
            }
            cx.notify();
        }
    }

    pub(super) fn handle_categories_dialog_type_change(
        &mut self,
        category_type: CategoryTypes,
        cx: &mut Context<'_, Self>,
    ) {
        let can_change_type = match self.dialog::<CategoriesDialog>() {
            Some(CategoriesDialog::Add { form, .. }) => form.parent_id.is_none(),
            Some(CategoriesDialog::Edit(id, _)) => self
                .categories(cx)
                .iter()
                .find(|c| c.id == *id)
                .is_some_and(|c| c.parent.is_none()),
            _ => false,
        };
        if !can_change_type {
            return;
        }
        let edited_id = match self.dialog::<CategoriesDialog>() {
            Some(CategoriesDialog::Edit(id, _)) => Some(*id),
            _ => None,
        };
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.category_type = Some(category_type.clone());
        }
        // For Edit dialogs, cascade the type change to descendants
        if let Some(id) = edited_id {
            let changed = self.ledger.update(cx, |store, cx| {
                store.apply(cx, lib_ledger::ChangeCategoryType { id, category_type })
            });
            self.report_category_write(changed);
        }
        cx.notify();
    }

    /// Confirms the open Categories dialog through the host's cycle: the form's command runs
    /// against the Ledger, a refusal stays on the form, and a
    /// landed one closes the dialog. Delete toasts the Splits it moved and keeps the selection in
    /// range.
    pub(super) fn confirm_categories_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = CategoriesApply {
            ledger: self.ledger.clone(),
            today: self.today,
        };
        let Some(dialog) = self.dialog_mut::<CategoriesDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        match outcome {
            CategoryOutcome::Saved => {}
            CategoryOutcome::Deleted { name, moved } => {
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_category_deleted(&name, moved),
                );
                // Keep the selection in range. It indexes the Settings rows, Expense then Income,
                // so the clamp counts those same rows rather than the Expense ones alone.
                let expanded = self.categories_view.read(cx).expanded().to_vec();
                let row_count = categories::settings_rows(self.categories(cx), &expanded).len();
                let selected = self.categories_view.read(cx).selected();
                self.categories_view.update(cx, |view, cx| {
                    view.set_selected(selected.min(row_count.saturating_sub(1)), cx)
                });
            }
        }
    }

    pub(super) fn handle_categories_disclosure_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.update_expanded_categories(cx, |expanded| {
            categories::toggle_expanded(expanded, id, false)
        });
        cx.notify();
    }

    /// A click on a row of Settings' Categories tree: selects it and moves focus into the page.
    pub(super) fn handle_settings_categories_row_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.select_category(Some(id), cx);
        self.focus_settings_page(cx);
        cx.notify();
    }
}
