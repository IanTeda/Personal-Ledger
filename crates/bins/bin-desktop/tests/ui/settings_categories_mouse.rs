//! The Settings Categories page driven by clicks: the index rail, the add button, row selection,
//! the row buttons and the dialogs' buttons. Only elements tagged with `debug_selector` are
//! clicked; state is read back from the Settings snapshot.

use super::common;
use bin_desktop::shell::router::SettingsCategoriesSnapshot;
use lib_core::CategoryId;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_categories(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Categories");
    assert_eq!(ui.settings_frame().page, "Categories");
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.click("settings-categories-add");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-category-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_add_dialog_confirm_is_inert_until_a_name_is_typed(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let before = ui
        .settings::<SettingsCategoriesSnapshot>()
        .category_names
        .len();

    ui.click("settings-categories-add");
    ui.click("add-category-confirm");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    ui.press("p e t s");
    ui.click("add-category-confirm");
    let categories_page = ui.settings::<SettingsCategoriesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(categories_page.category_names.len(), before + 1);
    assert!(
        categories_page
            .category_names
            .iter()
            .any(|name| name == "pets")
    );
}

#[gpui::test]
fn the_sub_button_opens_add_under_its_row(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    // Housing is a top-level row, so it can take a sub-category.
    ui.click(&format!("settings-categories-sub-{}", CategoryId::seed(1)));
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(ui.dialog_value("parent").as_deref(), Some("Housing"));
    ui.click("add-category-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn clicking_a_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>()
            .selected_category,
        None
    );

    // Food (id 6) is a top-level row, visible with the tree collapsed.
    ui.click(&format!("settings-categories-row-{}", CategoryId::seed(6)));
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>()
            .selected_category
            .as_deref(),
        Some("Food")
    );
}

#[gpui::test]
fn the_edit_action_opens_its_dialog_and_confirm_saves_the_rename(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.click(&format!("settings-categories-edit-{}", CategoryId::seed(6)));
    let categories_page = ui.settings::<SettingsCategoriesSnapshot>();
    assert_eq!(ui.dialog(), Some("EditCategory"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Food"));
    assert_eq!(categories_page.selected_category.as_deref(), Some("Food"));
    ui.press("x");
    ui.click("edit-category-confirm");
    let categories_page = ui.settings::<SettingsCategoriesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(categories_page.selected_category.as_deref(), Some("Foodx"));
    assert!(
        categories_page
            .category_names
            .iter()
            .any(|name| name == "Foodx")
    );
}

#[gpui::test]
fn the_edit_cancel_button_closes_without_renaming(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_names;

    ui.click(&format!("settings-categories-edit-{}", CategoryId::seed(6)));
    ui.press("x");
    ui.click("edit-category-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_names,
        before
    );
}

#[gpui::test]
fn the_delete_action_opens_its_dialog_and_cancel_keeps_the_row(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_names;

    // Household (id 10) is a leaf, so Delete opens its dialog.
    ui.click(&format!(
        "settings-categories-delete-{}",
        CategoryId::seed(10)
    ));
    assert_eq!(ui.dialog(), Some("DeleteCategory"));
    ui.click("delete-category-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_names,
        before
    );
}

#[gpui::test]
fn the_delete_confirm_is_inert_until_the_name_is_typed(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.click(&format!(
        "settings-categories-delete-{}",
        CategoryId::seed(10)
    ));
    ui.click("delete-category-confirm");
    assert_eq!(ui.dialog(), Some("DeleteCategory"));
    ui.press("shift-h o u s e h o l d");
    let commits = ui.commits();
    ui.click("delete-category-confirm");
    // Household is used by splits, so they move to Uncategorised: the count can stay level.
    let categories_page = ui.settings::<SettingsCategoriesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.commits(), commits + 1, "the click is one input");
    assert!(
        !categories_page
            .category_names
            .iter()
            .any(|name| name == "Household")
    );
}

#[gpui::test]
fn changing_the_type_then_cancel_leaves_the_category_and_its_descendants(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_types;

    // Food (id 6) is a top-level Expense with children.
    ui.click(&format!("settings-categories-edit-{}", CategoryId::seed(6)));
    ui.click("edit-type-income");
    assert_eq!(ui.dialog_value("type").as_deref(), Some("Income"));
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_types,
        before
    );
    ui.click("edit-category-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_types,
        before
    );
}

#[gpui::test]
fn changing_the_type_then_confirm_cascades_to_the_descendants(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let salary = CategoryId::seed(11);

    // Salary, a top-level Income no Budget or Bill Plan uses, takes a child first, so the change
    // has a subtree to cascade to.
    ui.click(&format!("settings-categories-sub-{salary}"));
    ui.press("b o n u s");
    ui.click("add-category-confirm");
    assert_eq!(ui.dialog(), None);
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_types;

    ui.click(&format!("settings-categories-edit-{salary}"));
    ui.click("edit-type-expense");
    ui.click("edit-category-confirm");
    assert_eq!(ui.dialog(), None);
    let after = ui.settings::<SettingsCategoriesSnapshot>().category_types;
    assert_ne!(after, before);
    for name in ["Salary", "bonus"] {
        assert!(
            after
                .iter()
                .any(|(n, t)| n == name && *t == lib_core::CategoryTypes::Expense),
            "{name} is an Expense: {after:?}"
        );
    }
    let changed = before.iter().filter(|row| !after.contains(row)).count();
    assert_eq!(changed, 2, "only Salary and its child changed type");
}

#[gpui::test]
fn a_blocked_type_shows_its_notice_on_click_and_the_confirm_is_refused_with_the_same_text(
    app: &mut TestAppContext,
) {
    let mut ui = on_categories(app);
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_types;

    // Food's Groceries is budgeted and named by a Bill Plan in the sample.
    ui.click(&format!("settings-categories-edit-{}", CategoryId::seed(6)));
    ui.click("edit-type-income");
    let notice = ui.dialog_value("notice").expect("a notice");
    assert!(
        notice.starts_with("Food can't become Income: ")
            && notice.ends_with(" use its Categories."),
        "{notice}"
    );

    ui.click("edit-category-confirm");
    assert_eq!(ui.dialog(), Some("EditCategory"), "the Dialog stays open");
    assert_eq!(ui.dialog_value("error"), Some(notice));
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_types,
        before
    );

    // Picking the type it has again clears the notice.
    ui.click("edit-type-expense");
    let notice = ui.dialog_value("notice").unwrap_or_default();
    assert!(!notice.contains("can't become"), "{notice}");
    ui.click("edit-category-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_sub_button_on_a_budgeted_leaf_warns_and_the_confirm_stops_its_budget(
    app: &mut TestAppContext,
) {
    let mut ui = on_categories(app);
    let transport = CategoryId::seed(9);

    ui.click(&format!("settings-categories-sub-{transport}"));
    assert_eq!(ui.dialog_value("parent").as_deref(), Some("Transport"));
    let notice = ui.dialog_value("notice").expect("a notice");
    assert!(
        notice.starts_with("Transport' budget stops from"),
        "{notice}"
    );
    ui.press("f u e l");
    ui.click("add-category-confirm");

    assert_eq!(ui.dialog(), None);
    ui.press("g b");
    let row = ui
        .budgets()
        .rows
        .into_iter()
        .find(|row| row.category == "Transport");
    assert!(!row.as_ref().is_some_and(|row| row.budgeted), "{row:?}");
}

#[gpui::test]
fn the_sub_button_on_an_unbudgeted_leaf_shows_no_notice_and_stops_nothing(
    app: &mut TestAppContext,
) {
    let mut ui = on_categories(app);
    let budgets = ui.budgets().rows;

    // Salary is an Income leaf, so no Budget holds an amount for it.
    ui.click(&format!("settings-categories-sub-{}", CategoryId::seed(11)));
    assert_eq!(ui.dialog_value("parent").as_deref(), Some("Salary"));
    assert!(
        !ui.dialog_value("notice")
            .expect("a notice")
            .contains("stops from")
    );
    ui.press("b o n u s");
    ui.click("add-category-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.budgets().rows, budgets);
}

#[gpui::test]
fn the_sub_button_on_a_leaf_with_splits_warns_and_the_confirm_hands_them_over(
    app: &mut TestAppContext,
) {
    let mut ui = on_categories(app);
    let groceries = CategoryId::seed(7);

    ui.click(&format!("settings-categories-sub-{groceries}"));
    assert!(
        ui.dialog_value("parent")
            .is_some_and(|parent| parent.ends_with("Groceries"))
    );
    ui.press("s u p e r m a r k e t");
    let notice = ui.dialog_value("notice").expect("a notice");
    assert!(notice.ends_with("move to supermarket."), "{notice}");
    ui.click("add-category-confirm");

    assert_eq!(ui.dialog(), None);
}

/// Deletes Household (which has Splits) by clicks, so the Ledger makes its Expense Uncategorised,
/// and returns that row's id.
fn make_uncategorised(ui: &mut Harness<'_>) -> Option<CategoryId> {
    ui.click(&format!(
        "settings-categories-delete-{}",
        CategoryId::seed(10)
    ));
    ui.press("shift-h o u s e h o l d");
    ui.click("delete-category-confirm");
    assert_eq!(ui.dialog(), None);
    let page = ui.settings::<SettingsCategoriesSnapshot>();
    page.category_names
        .iter()
        .zip(&page.category_ids)
        .find(|(name, _)| *name == "Uncategorised")
        .map(|(_, id)| *id)
}

#[gpui::test]
fn the_edit_action_on_uncategorised_opens_it_read_only_and_a_type_click_changes_nothing(
    app: &mut TestAppContext,
) {
    let mut ui = on_categories(app);
    let id = make_uncategorised(&mut ui).expect("Household's Splits moved to a new Uncategorised");

    ui.click(&format!("settings-categories-edit-{id}"));
    assert_eq!(ui.dialog(), Some("EditCategory"));
    assert_eq!(
        ui.dialog_value("read_only").as_deref(),
        Some("name,parent,type")
    );
    ui.click("edit-type-income");
    assert_eq!(ui.dialog_value("type").as_deref(), Some("Expense"));
    ui.click("edit-category-confirm");

    assert_eq!(ui.dialog(), None);
    let page = ui.settings::<SettingsCategoriesSnapshot>();
    assert!(page.category_types.contains(&(
        "Uncategorised".to_string(),
        lib_core::CategoryTypes::Expense
    )));
}

#[gpui::test]
fn the_sub_button_on_uncategorised_is_refused_on_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let id = make_uncategorised(&mut ui).expect("Household's Splits moved to a new Uncategorised");

    ui.click(&format!("settings-categories-sub-{id}"));

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_frame().status_message.as_deref(),
        Some("Uncategorised is fixed, so it takes no subcategories")
    );
}

#[gpui::test]
fn the_add_buttons_parent_list_leaves_uncategorised_out(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    assert!(make_uncategorised(&mut ui).is_some());

    ui.click("settings-categories-add");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    let parents = ui.dialog_value("parents").expect("the offered parents");
    assert!(!parents.split(',').any(|name| name == "Uncategorised"));
    ui.click("add-category-cancel");
}
