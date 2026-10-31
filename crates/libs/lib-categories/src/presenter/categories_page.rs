//! The Categories Settings page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2i): the
//! Expense tree then the Income tree, each under its kicker, with NAME and ACTIONS only and
//! `+ sub · edit · delete` per row. Budget and spend figures live in Budgets and Reports.
//!
//! The selection is a Category id, so it follows its row across a rename or a move. Nothing is
//! selected until the first move, which lands on the first row; a Category that has gone leaves
//! nothing selected. The folds are the parents shown open, and movement walks only the rows they
//! show, Expense then Income.

use lib_core::{CategoryId, CategoryTypes};
use lib_list_cursor::{Step, step_id};
use lib_locale::format::upper;
use lib_toast::Notice;
use lib_viper::keymap::{DELETE, EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};
use lib_viper::view::Hint;

use super::count;
use crate::keymap::{COLLAPSE, EXPAND, NEW_CHILD};
use crate::protocols::{CategoriesDialog, CategoriesOutcome, CategoriesRouter, SharedCategories};
use crate::{
    Category, TreeNode, child_count, get, is_leaf, levels_deep, msg, settings_rows, toggle_expanded,
};

/// The parents open when the page is first drawn: the stub tree's Housing, Utilities and Food, as
/// the handoff draws it.
const OPEN_AT_START: [CategoryId; 3] = [
    CategoryId::seed(1),
    CategoryId::seed(3),
    CategoryId::seed(6),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoriesPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u` and their arrow, Home/End and Page keys.
    Move(Step),
    /// A click on a row.
    Select(CategoryId),
    /// New opens Add at top level; NewChild opens it under the selected Category.
    New,
    NewChild,
    /// Edit and Delete act on the selected Category.
    Edit,
    Delete,
    /// A row's own **+ sub**, **edit** or **delete**.
    AddChildRow(CategoryId),
    EditRow(CategoryId),
    DeleteRow(CategoryId),
    /// A click on a parent's ▾/▸.
    Toggle(CategoryId),
    /// `right` opens a folded parent, or steps into an open one's first child.
    Expand,
    /// `left` folds an open parent, or climbs from a nested row to its parent.
    Collapse,
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<CategoriesOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoriesPageViewModel {
    /// The heading's meta and the status line's right-hand text: `12 categories · 3 levels deep`.
    pub scope: String,
    pub expense: CategoryTree,
    pub income: CategoryTree,
    /// Over the Expense tree only.
    pub add_button: String,
    pub columns: CategoryColumns,
    pub note: String,
    pub row_add_child: String,
    pub row_edit: String,
    pub row_delete: String,
    pub selected: Option<CategoryId>,
    pub hints: Vec<Hint>,
    /// The Dialog the page last opened, until it closes.
    pub open_dialog: Option<CategoriesDialog>,
}

/// One type's tree: its kicker (`EXPENSE · 10`) and the rows its folds show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryTree {
    pub kicker: String,
    pub rows: Vec<CategoryLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryColumns {
    pub name: String,
    pub actions: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLine {
    pub id: CategoryId,
    pub name: String,
    /// 0 for top level.
    pub depth: u32,
    pub is_leaf: bool,
    pub is_expanded: bool,
    /// A parent's `3 subcategories`.
    pub subcount: Option<String>,
    /// Whether the row offers **+ sub**: it sits above the depth cap.
    pub can_add_child: bool,
}

pub struct CategoriesPagePresenter {
    categories: SharedCategories,
    keymap: Keymap<CategoriesPageEvent>,
    selected: Option<CategoryId>,
    expanded: Vec<CategoryId>,
    open_dialog: Option<CategoriesDialog>,
}

impl CategoriesPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(categories: SharedCategories, keymap: Keymap<CategoriesPageEvent>) -> Self {
        Self {
            categories,
            keymap,
            selected: None,
            expanded: OPEN_AT_START.to_vec(),
            open_dialog: None,
        }
    }

    /// Whether the page takes `event` from its keymap, rather than the Client's frame having it:
    /// `left` with nothing to fold or climb to is the Settings index's again, and Edit, Delete and
    /// New child with no Category selected have nothing to act on.
    pub fn takes(&self, event: &CategoriesPageEvent) -> bool {
        match event {
            CategoriesPageEvent::Collapse => self.collapse_target().is_some(),
            CategoriesPageEvent::Edit
            | CategoriesPageEvent::Delete
            | CategoriesPageEvent::NewChild => self.selected().is_some(),
            _ => true,
        }
    }

    pub fn handle(&mut self, event: CategoriesPageEvent, router: &mut dyn CategoriesRouter) {
        match event {
            CategoriesPageEvent::Move(step) => self.step(step),
            CategoriesPageEvent::Select(id) => self.select(id),
            CategoriesPageEvent::New => {
                self.open(CategoriesDialog::AddCategory { parent: None }, router)
            }
            CategoriesPageEvent::NewChild => {
                if let Some(id) = self.selected() {
                    self.open(CategoriesDialog::AddCategory { parent: Some(id) }, router);
                }
            }
            CategoriesPageEvent::Edit => {
                if let Some(id) = self.selected() {
                    self.open(CategoriesDialog::EditCategory(id), router);
                }
            }
            CategoriesPageEvent::Delete => {
                if let Some(id) = self.selected() {
                    self.delete(id, router);
                }
            }
            CategoriesPageEvent::AddChildRow(id) => {
                self.select(id);
                self.open(CategoriesDialog::AddCategory { parent: Some(id) }, router);
            }
            CategoriesPageEvent::EditRow(id) => {
                if self.exists(id) {
                    self.select(id);
                    self.open(CategoriesDialog::EditCategory(id), router);
                }
            }
            CategoriesPageEvent::DeleteRow(id) => self.delete(id, router),
            CategoriesPageEvent::Toggle(id) => toggle_expanded(&mut self.expanded, id, false),
            CategoriesPageEvent::Expand => self.expand(),
            CategoriesPageEvent::Collapse => self.collapse(),
            CategoriesPageEvent::DialogClosed(_) => self.open_dialog = None,
        }
    }

    pub fn view_model(&self) -> CategoriesPageViewModel {
        let categories = self.categories.borrow().categories();
        let rows = settings_rows(&categories, &self.expanded);
        let tree = |wanted: CategoryTypes, kicker: fn(&str) -> String| {
            let total = categories
                .iter()
                .filter(|category| category.category_type == wanted)
                .count();
            CategoryTree {
                kicker: upper(&kicker(&total.to_string())),
                rows: rows
                    .iter()
                    .filter(|row| {
                        get(&categories, row.id).is_some_and(|c| c.category_type == wanted)
                    })
                    .map(|row| line(&categories, row))
                    .collect(),
            }
        };
        CategoriesPageViewModel {
            scope: msg::categories_summary(
                count(categories.len()),
                i64::from(levels_deep(&categories)),
            ),
            expense: tree(CategoryTypes::Expense, msg::categories_kicker_expense),
            income: tree(CategoryTypes::Income, msg::categories_kicker_income),
            add_button: msg::categories_add_button(),
            columns: CategoryColumns {
                name: upper(&lib_locale::msg::column_name()),
                actions: upper(&lib_locale::msg::column_actions()),
            },
            note: msg::categories_note(),
            row_add_child: msg::categories_row_sub(),
            row_edit: msg::categories_row_edit(),
            row_delete: msg::categories_row_delete(),
            selected: self.selected(),
            hints: self.hints(),
            open_dialog: self.open_dialog,
        }
    }

    fn categories(&self) -> Vec<Category> {
        self.categories.borrow().categories()
    }

    fn exists(&self, id: CategoryId) -> bool {
        self.categories.borrow().category(id).is_some()
    }

    /// The selected Category, while it still exists.
    fn selected(&self) -> Option<CategoryId> {
        self.selected.filter(|id| self.exists(*id))
    }

    /// Selects Category `id`, if it still exists.
    fn select(&mut self, id: CategoryId) {
        if self.exists(id) {
            self.selected = Some(id);
        }
    }

    /// Walks the rows the folds show; with nothing selected the first step lands on the first row.
    fn step(&mut self, step: Step) {
        let ids: Vec<CategoryId> = settings_rows(&self.categories(), &self.expanded)
            .iter()
            .map(|row| row.id)
            .collect();
        if !ids.is_empty() {
            self.selected = step_id(&ids, self.selected(), step);
        }
    }

    /// Only a leaf is deleted; a parent says why on the status line and opens nothing.
    fn delete(&mut self, id: CategoryId, router: &mut dyn CategoriesRouter) {
        if !is_leaf(&self.categories(), id) {
            router.notify(Notice::Status(
                msg::categories_status_delete_children_first(),
            ));
            return;
        }
        self.open(CategoriesDialog::DeleteCategory(id), router);
    }

    fn expand(&mut self) {
        let Some(id) = self.selected() else { return };
        if is_leaf(&self.categories(), id) {
            return;
        }
        if self.expanded.contains(&id) {
            self.step(Step::Next);
        } else {
            self.expanded.push(id);
        }
    }

    fn collapse(&mut self) {
        match self.collapse_target() {
            Some(Collapse::Fold(id)) => self.expanded.retain(|open| *open != id),
            Some(Collapse::Climb(parent)) => self.selected = Some(parent),
            None => {}
        }
    }

    /// What `left` does from the selected row: fold it while it is an open parent, else climb to
    /// its parent.
    fn collapse_target(&self) -> Option<Collapse> {
        let id = self.selected()?;
        let categories = self.categories();
        if self.expanded.contains(&id) && !is_leaf(&categories, id) {
            return Some(Collapse::Fold(id));
        }
        get(&categories, id)?.parent.map(Collapse::Climb)
    }

    fn open(&mut self, dialog: CategoriesDialog, router: &mut dyn CategoriesRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    fn chord(&self, command: &str) -> &str {
        self.keymap.chord(command).unwrap_or_default()
    }

    fn hints(&self) -> Vec<Hint> {
        let hint = |chord: String, action: String| Hint { chord, action };
        let pair = |a: &str, b: &str| format!("{}/{}", self.chord(a), self.chord(b));
        vec![
            hint(pair(MOVE_DOWN, MOVE_UP), msg::categories_hint_row()),
            hint(pair(EXPAND, COLLAPSE), msg::categories_hint_expand()),
            hint(self.chord(EDIT).to_string(), msg::categories_hint_edit()),
            hint(
                self.chord(DELETE).to_string(),
                msg::categories_hint_delete(),
            ),
            hint(self.chord(NEW).to_string(), msg::categories_hint_new()),
            hint(
                self.chord(NEW_CHILD).to_string(),
                msg::categories_hint_sub(),
            ),
        ]
    }
}

/// What `left` does.
enum Collapse {
    Fold(CategoryId),
    Climb(CategoryId),
}

fn line(categories: &[Category], row: &TreeNode) -> CategoryLine {
    CategoryLine {
        id: row.id,
        name: row.name.clone(),
        depth: row.depth,
        is_leaf: row.is_leaf,
        is_expanded: row.is_expanded,
        subcount: (!row.is_leaf)
            .then(|| msg::categories_subcount(count(child_count(categories, row.id)))),
        can_add_child: row.depth < 2,
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::keymap::categories_page_keymap;
    use crate::protocols::CategoryChange;

    const HOUSING: CategoryId = CategoryId::seed(1);
    const RENT: CategoryId = CategoryId::seed(2);
    const UTILITIES: CategoryId = CategoryId::seed(3);
    const FOOD: CategoryId = CategoryId::seed(6);
    const HOUSEHOLD: CategoryId = CategoryId::seed(10);

    struct Page {
        presenter: CategoriesPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new(shared: SharedCategories) -> Self {
            Self::with_keys(shared, &KeyBindingConfig::default())
        }

        fn with_keys(shared: SharedCategories, config: &KeyBindingConfig) -> Self {
            Self {
                presenter: CategoriesPagePresenter::new(shared, categories_page_keymap(config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: CategoriesPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> CategoriesPageViewModel {
            self.presenter.view_model()
        }

        fn selected_name(&self) -> Option<String> {
            let view = self.view();
            view.expense
                .rows
                .into_iter()
                .chain(view.income.rows)
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name)
        }
    }

    fn open(dialog: CategoriesDialog) -> Vec<RouterCall> {
        vec![RouterCall::OpenDialog(dialog)]
    }

    #[test]
    fn the_trees_run_expense_then_income_under_their_kickers_and_the_scope() {
        let (_, shared) = stub();
        let view = Page::new(shared).view();

        assert_eq!(view.scope, "12 categories \u{b7} 3 levels deep");
        assert_eq!(view.expense.kicker, "EXPENSE \u{b7} 10");
        assert_eq!(view.income.kicker, "INCOME \u{b7} 2");
        let names: Vec<&str> = view
            .expense
            .rows
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        // Housing, Utilities and Food start open; Transport and Household are leaves.
        assert_eq!(
            names,
            [
                "Housing",
                "Rent",
                "Utilities",
                "Electricity",
                "Water",
                "Food",
                "Groceries",
                "Dining",
                "Transport",
                "Household"
            ]
        );
        let housing = &view.expense.rows[0];
        assert_eq!(housing.subcount.as_deref(), Some("2 subcategories"));
        assert!(housing.can_add_child && housing.is_expanded);
        let electricity = &view.expense.rows[3];
        assert_eq!((electricity.depth, electricity.can_add_child), (2, false));
        assert_eq!(view.columns.name, "NAME");
        assert_eq!(
            view.selected, None,
            "nothing is selected until the first move"
        );
    }

    #[test]
    fn movement_walks_the_shown_rows_and_the_folds_open_and_close() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        page.send(CategoriesPageEvent::Move(Step::Next));
        assert_eq!(page.selected_name().as_deref(), Some("Housing"));
        page.send(CategoriesPageEvent::Move(Step::Last));
        assert_eq!(page.selected_name().as_deref(), Some("Interest"));

        page.send(CategoriesPageEvent::Select(HOUSING));
        page.send(CategoriesPageEvent::Collapse);
        assert!(
            !page.view().expense.rows[0].is_expanded,
            "left folds Housing"
        );
        page.send(CategoriesPageEvent::Move(Step::Next));
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Food"),
            "Rent is folded away"
        );
        page.send(CategoriesPageEvent::Select(HOUSING));
        page.send(CategoriesPageEvent::Expand);
        page.send(CategoriesPageEvent::Expand);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Rent"),
            "right steps in"
        );

        assert!(page.presenter.takes(&CategoriesPageEvent::Collapse));
        page.send(CategoriesPageEvent::Collapse);
        assert_eq!(
            page.selected_name().as_deref(),
            Some("Housing"),
            "left climbs"
        );
        page.send(CategoriesPageEvent::Toggle(HOUSING));
        assert!(
            !page.presenter.takes(&CategoriesPageEvent::Collapse),
            "a folded top-level row leaves left to Settings"
        );
    }

    #[test]
    fn new_edit_and_delete_open_their_dialogs_on_the_selected_category() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);

        assert_eq!(page.send(CategoriesPageEvent::Edit), []);
        assert_eq!(
            page.send(CategoriesPageEvent::New),
            open(CategoriesDialog::AddCategory { parent: None })
        );
        assert_eq!(
            page.view().open_dialog,
            Some(CategoriesDialog::AddCategory { parent: None })
        );
        page.send(CategoriesPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);

        page.send(CategoriesPageEvent::Select(UTILITIES));
        assert_eq!(
            page.send(CategoriesPageEvent::NewChild),
            open(CategoriesDialog::AddCategory {
                parent: Some(UTILITIES)
            })
        );
        assert_eq!(
            page.send(CategoriesPageEvent::Edit),
            open(CategoriesDialog::EditCategory(UTILITIES))
        );
        assert_eq!(
            page.send(CategoriesPageEvent::DeleteRow(HOUSEHOLD)),
            open(CategoriesDialog::DeleteCategory(HOUSEHOLD))
        );
        assert_eq!(
            page.send(CategoriesPageEvent::AddChildRow(FOOD)),
            open(CategoriesDialog::AddCategory { parent: Some(FOOD) })
        );
        assert_eq!(page.selected_name().as_deref(), Some("Food"));
    }

    #[test]
    fn delete_on_a_parent_says_why_on_the_status_line_and_opens_nothing() {
        let (_, shared) = stub();
        let mut page = Page::new(shared);
        page.send(CategoriesPageEvent::Select(HOUSING));

        assert_eq!(
            page.send(CategoriesPageEvent::Delete),
            [RouterCall::Notice(Notice::Status(
                "delete or move its children first".into()
            ))]
        );
        assert_eq!(page.view().open_dialog, None);
    }

    #[test]
    fn a_deleted_selection_leaves_nothing_selected() {
        let (categories, shared) = stub();
        let mut page = Page::new(shared);
        page.send(CategoriesPageEvent::Select(RENT));

        page.send(CategoriesPageEvent::DialogClosed(Some(CategoriesOutcome {
            change: CategoryChange::Deleted,
            id: RENT,
            name: "Rent".into(),
        })));
        categories.borrow_mut().categories.retain(|c| c.id != RENT);

        assert_eq!(page.view().selected, None);
        assert_eq!(page.send(CategoriesPageEvent::Edit), []);
    }

    #[test]
    fn the_legend_shows_the_chords_the_keymap_binds() {
        let (_, shared) = stub();
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert("categories.new_child".into(), "s".into());
        let view = Page::with_keys(shared, &config).view();

        let chords: Vec<&str> = view.hints.iter().map(|hint| hint.chord.as_str()).collect();
        assert_eq!(chords, ["j/k", "right/left", "e", "d", "n", "s"]);
    }
}
