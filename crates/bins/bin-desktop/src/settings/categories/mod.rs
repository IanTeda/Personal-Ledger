//! The **Categories** Settings page (ADR-0037, and its amendment's one folder per page): the
//! nested Category tree, Expense then Income. [`CategoriesPage`] reads the tree straight from
//! [`LedgerStore`], keeps its own cursor and folds, renders itself (`render.rs`), builds the Add,
//! Edit and Delete Dialogs (`dialogs/`) and reaches `Root` only through [`ViewEvent`]; `Root`'s
//! Dialog host draws them and runs their confirm cycle through `apply.rs`. The Settings View
//! forwards its keys, movement and status to it while the page has focus.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use crate::settings::SettingsPage;
use crate::settings::{
    page::PageRowClicked,
    pages::{Page, focus_on_row_click},
    state::SettingsView,
};
use chrono::NaiveDate;
use gpui::{AnyElement, IntoElement as _};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_categories::{Category, is_leaf, settings_rows, toggle_expanded};
use lib_core::{CategoryId, CategoryTypes};
use lib_ledger::Domain;
use lib_list_cursor::step_id;

use crate::{
    budgets::PERSONAL_SPENDING_ID,
    form::field::TextField,
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::hints::settings_categories_hints,
    shell::view::{
        dialog_host::open_dialog,
        statusline::{Hint, PageStatus, owned_hints},
    },
    view::event::ViewEvent,
};
use dialogs::CategoriesDialog;
use form::{BudgetLock, CategoryField, CategoryForm, DeleteCategoryForm};

impl EventEmitter<ViewEvent> for CategoriesPage {}

impl EventEmitter<PageRowClicked> for CategoriesPage {}

/// The Categories page's own state: the cursor, the selected Category and the parents shown open.
pub struct CategoriesPage {
    ledger: Entity<LedgerStore>,
    /// "Today" for the Monthly budget an Edit dialog shows, read once when the Shell is built.
    today: NaiveDate,
    /// The Category the cursor is on. It may name a Category that no longer exists, so read it
    /// through [`Self::live_selected_id`].
    selected_id: Option<CategoryId>,
    /// The parents shown open: Housing, Utilities and Food by default.
    expanded: Vec<CategoryId>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the tree changes.
    _categories_changed: Subscription,
}

impl CategoriesPage {
    pub fn new(
        ledger: Entity<LedgerStore>,
        clock: &lib_core::Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let today = clock.today();
        let _categories_changed = notify_on(&ledger, Domain::Categories, cx);
        Self {
            ledger,
            today,
            selected_id: None,
            expanded: vec![
                CategoryId::seed(1),
                CategoryId::seed(3),
                CategoryId::seed(6),
            ],
            page_focused: false,
            _categories_changed,
        }
    }

    pub(crate) fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.ledger.read(cx).ledger().categories()
    }

    /// The selected Category, if it still exists.
    fn live_selected_id(&self, cx: &App) -> Option<CategoryId> {
        self.selected_id
            .filter(|id| self.categories(cx).iter().any(|c| c.id == *id))
    }

    /// The heading's meta: `12 categories · 3 levels deep`.
    pub(crate) fn scope_note(&self, cx: &App) -> String {
        render::scope_text(self.categories(cx))
    }
}

impl Page for CategoriesPage {
    const PAGE: SettingsPage = SettingsPage::Categories;

    /// `left` folds an open parent, or climbs from a nested row to its parent. Settings would
    /// otherwise read it as a move back to the index, so the page takes it ahead of Settings'
    /// own keys only when it has something to do.
    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "left" && !keystroke.modifiers.shift && self.left_is_local(cx) {
            self.step_fold(false, cx);
            return true;
        }
        false
    }

    /// `n` adds a top-level Category, `N` a sub-Category under the selected one, `e` edits and `d`
    /// deletes the selected one, and `right` opens a folded parent. `left` is taken by [`Self::key_ahead`].
    fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        let shift = modifiers.shift;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }

        let selected_id = self.live_selected_id(cx);
        match keystroke.key.as_str() {
            "n" => {
                if shift {
                    if let Some(id) = selected_id {
                        self.open_add_dialog(Some(id), cx);
                    }
                } else {
                    self.open_add_dialog(None, cx);
                }
                true
            }
            "e" => {
                if let Some(id) = selected_id {
                    self.open_edit_dialog(id, cx);
                }
                true
            }
            "d" => {
                if let Some(id) = selected_id {
                    if !is_leaf(self.categories(cx), id) {
                        cx.emit(ViewEvent::SetStatus(Some(
                            crate::msg::desktop_status_delete_children_first(),
                        )));
                    } else {
                        self.open_delete_dialog(id, cx);
                    }
                }
                true
            }
            "right" => {
                self.step_fold(true, cx);
                true
            }
            _ => false,
        }
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the tree's visible rows, Expense then Income. With no
    /// row selected (or the selected one folded away) the first press lands on the first row.
    fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        let Some(step) = movement.step() else {
            return true;
        };
        let ids: Vec<CategoryId> = settings_rows(self.categories(cx), &self.expanded)
            .iter()
            .map(|row| row.id)
            .collect();
        if ids.is_empty() {
            return true;
        }
        let next = step_id(&ids, self.selected_id, step);
        self.select(next, cx);
        true
    }

    /// The fixed legend, and the heading's meta.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: owned_hints(settings_categories_hints()),
            right: self.scope_note(cx),
        })
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        owned_hints(settings_categories_hints())
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.scope_note(cx).into_any_element()
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}

impl CategoriesPage {
    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// **+ Add category** opens the Add dialog at the top level.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog(None, cx);
    }

    /// A row's **+ sub** selects that row and opens the Add dialog under it.
    pub(crate) fn click_add_sub(&mut self, parent_id: CategoryId, cx: &mut Context<'_, Self>) {
        self.select(Some(parent_id), cx);
        self.open_add_dialog(Some(parent_id), cx);
    }

    pub(crate) fn click_edit(&mut self, category_id: CategoryId, cx: &mut Context<'_, Self>) {
        if self.categories(cx).iter().any(|c| c.id == category_id) {
            self.select(Some(category_id), cx);
            self.open_edit_dialog(category_id, cx);
        }
    }

    pub(crate) fn click_delete(&mut self, category_id: CategoryId, cx: &mut Context<'_, Self>) {
        if !is_leaf(self.categories(cx), category_id) {
            cx.emit(ViewEvent::SetStatus(Some(
                crate::msg::desktop_status_delete_children_first(),
            )));
            return;
        }
        self.open_delete_dialog(category_id, cx);
    }

    pub(crate) fn click_disclosure(&mut self, id: CategoryId, cx: &mut Context<'_, Self>) {
        self.expanded_edit(cx, |expanded| toggle_expanded(expanded, id, false));
    }

    /// A click on a row selects it and hands focus to the page.
    pub(crate) fn click_row(&mut self, id: CategoryId, cx: &mut Context<'_, Self>) {
        self.select(Some(id), cx);
        cx.emit(PageRowClicked);
    }

    // -----------------------------------------------------------------------------------------
    // Movement, folds and the cursor
    // -----------------------------------------------------------------------------------------

    /// Whether `left` has something to do inside the tree: fold an open parent, or climb from a
    /// nested row to its parent.
    fn left_is_local(&self, cx: &App) -> bool {
        let Some(id) = self.selected_id else {
            return false;
        };
        let categories = self.categories(cx);
        let open_parent = self.expanded.contains(&id) && !is_leaf(categories, id);
        open_parent
            || categories
                .iter()
                .any(|category| category.id == id && category.parent.is_some())
    }

    /// `right` opens a folded parent, or steps into an open one's first child. `left` folds an open
    /// parent, or climbs to the parent of a nested row.
    fn step_fold(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        let Some(id) = self.selected_id else {
            return;
        };
        let is_parent = !is_leaf(self.categories(cx), id);
        let open = self.expanded.contains(&id);
        if forward {
            if !is_parent {
                return;
            }
            if open {
                self.apply_movement(Movement::Next, cx);
            } else {
                self.expanded_edit(cx, |expanded| expanded.push(id));
            }
        } else if is_parent && open {
            self.expanded_edit(cx, |expanded| expanded.retain(|&other| other != id));
        } else if let Some(parent) = self
            .categories(cx)
            .iter()
            .find(|category| category.id == id)
            .and_then(|category| category.parent)
        {
            self.select(Some(parent), cx);
        }
    }

    fn select(&mut self, id: Option<CategoryId>, cx: &mut Context<'_, Self>) {
        self.selected_id = id;
        cx.notify();
    }

    fn expanded_edit(
        &mut self,
        cx: &mut Context<'_, Self>,
        edit: impl FnOnce(&mut Vec<CategoryId>),
    ) {
        edit(&mut self.expanded);
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    /// Why the Monthly budget field is read-only for `category_id` (`None` for a Category being
    /// added): the Personal spending Budget it reads and writes is archived, or the Category is a
    /// parent and so only rolls up.
    pub(crate) fn budget_lock(
        &self,
        category_id: Option<CategoryId>,
        cx: &App,
    ) -> Option<BudgetLock> {
        if category_id.is_some_and(|id| !is_leaf(self.categories(cx), id)) {
            return Some(BudgetLock::Parent);
        }
        self.ledger
            .read(cx)
            .ledger()
            .budgets()
            .get(PERSONAL_SPENDING_ID)
            .filter(|budget| budget.is_archived())
            .map(|budget| BudgetLock::Archived(budget.name.clone()))
    }

    /// Opens the Add dialog under `parent_id`, or at the top level. A child takes its parent's type,
    /// locked in the form; a top-level one starts as Expense.
    fn open_add_dialog(&mut self, parent_id: Option<CategoryId>, cx: &mut Context<'_, Self>) {
        let category_type = match parent_id {
            Some(id) => self
                .categories(cx)
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.category_type.clone()),
            None => Some(CategoryTypes::Expense),
        };
        let form = CategoryForm {
            name: TextField::default(),
            parent_id,
            category_type,
            budget: TextField::default(),
            budget_lock: self.budget_lock(None, cx),
            focused: CategoryField::Name,
            error: None,
            today: self.today,
        };
        self.open(CategoriesDialog::Add { parent_id, form }, cx);
    }

    /// Opens the Edit dialog on `category_id`, pre-filled. Monthly budget shows the current month's
    /// amount in the Personal spending Budget (a parent's is its children's sum).
    fn open_edit_dialog(&mut self, category_id: CategoryId, cx: &mut Context<'_, Self>) {
        let Some(category) = self
            .categories(cx)
            .iter()
            .find(|c| c.id == category_id)
            .cloned()
        else {
            return;
        };
        let budget = self
            .ledger
            .read(cx)
            .ledger()
            .budgets()
            .monthly_limit(
                PERSONAL_SPENDING_ID,
                self.categories(cx),
                category_id,
                self.today,
            )
            .map(|amount| amount.0.to_string())
            .unwrap_or_default();
        let form = CategoryForm {
            name: TextField::new(category.name.as_str()),
            parent_id: category.parent,
            category_type: Some(category.category_type.clone()),
            budget: TextField::new(budget),
            budget_lock: self.budget_lock(Some(category_id), cx),
            focused: CategoryField::Name,
            error: None,
            today: self.today,
        };
        self.open(CategoriesDialog::Edit(category_id, form), cx);
    }

    /// Opens the Delete dialog on `category_id`, copying its name in so the form validates without
    /// `Root`. A no-op if the Category is gone.
    fn open_delete_dialog(&mut self, category_id: CategoryId, cx: &mut Context<'_, Self>) {
        let Some(category) = self.categories(cx).iter().find(|c| c.id == category_id) else {
            return;
        };
        let form = DeleteCategoryForm::new(category.name.as_str());
        self.open(CategoriesDialog::Delete(category_id, form), cx);
    }

    /// Asks the Dialog host to open `dialog`. The page keeps its cursor through a confirm: a
    /// save leaves it on its row and a Delete's Toast is the host's, so the outcome lands on
    /// nothing here.
    fn open(&self, dialog: CategoriesDialog, cx: &mut Context<'_, Self>) {
        open_dialog(dialog, |_page: &mut Self, _outcome, _cx| {}, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Categories' names in storage order, and the selected row's name.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsCategoriesSnapshot {
        let categories = self.categories(cx);
        SettingsCategoriesSnapshot {
            category_names: categories.iter().map(|c| c.name.clone()).collect(),
            category_types: categories
                .iter()
                .map(|c| (c.name.clone(), c.category_type.clone()))
                .collect(),
            selected_category: self.live_selected_id(cx).and_then(|id| {
                categories
                    .iter()
                    .find(|category| category.id == id)
                    .map(|category| category.name.clone())
            }),
        }
    }
}

/// The Categories page: names and types in storage order, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsCategoriesSnapshot {
    pub category_names: Vec<String>,
    /// Each Category's name and type, in storage order.
    pub category_types: Vec<(String, CategoryTypes)>,
    pub selected_category: Option<String>,
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_core::CategoryTypes;
    use lib_ledger::AddCategory;

    use super::*;
    use crate::ledger_store::{LedgerChanged, test_ledger};

    /// A Ledger over the seeded tree.
    fn seeded_ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            lib_ledger::LedgerParts {
                categories: lib_categories::default_categories(),
                ..lib_ledger::LedgerParts::default()
            },
        )
    }

    fn add(ledger: &Entity<LedgerStore>, cx: &mut TestAppContext, parent: CategoryId) -> bool {
        ledger.update(cx, |store, cx| {
            store
                .apply(
                    cx,
                    AddCategory {
                        name: "Snacks".into(),
                        parent_id: Some(parent),
                        category_type: CategoryTypes::Expense,
                        monthly: None,
                    },
                )
                .is_ok()
        })
    }

    #[gpui::test]
    fn a_landed_write_names_the_categories_domain_and_a_refused_one_does_not(
        cx: &mut TestAppContext,
    ) {
        let ledger = seeded_ledger(cx);
        let announced = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = announced.clone();
        cx.update(|cx| {
            cx.subscribe(&ledger, move |_, event: &LedgerChanged, _| {
                sink.borrow_mut().push(event.what.clone())
            })
            .detach();
        });

        // Salary (11) is Income, so an Expense child under it is refused.
        assert!(!add(&ledger, cx, CategoryId::seed(11)));
        cx.run_until_parked();
        assert!(announced.borrow().is_empty());

        assert!(add(&ledger, cx, CategoryId::seed(6)));
        cx.run_until_parked();
        assert_eq!(
            *announced.borrow(),
            vec![std::collections::BTreeSet::from([Domain::Categories])]
        );
    }
}
