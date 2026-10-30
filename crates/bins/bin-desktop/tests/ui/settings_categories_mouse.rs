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
    ui.click("delete-category-confirm");
    // Household is used by splits, so they move to Uncategorised: the count can stay level.
    let categories_page = ui.settings::<SettingsCategoriesSnapshot>();
    assert_eq!(ui.dialog(), None);
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
    let before = ui.settings::<SettingsCategoriesSnapshot>().category_types;

    ui.click(&format!("settings-categories-edit-{}", CategoryId::seed(6)));
    ui.click("edit-type-income");
    ui.click("edit-category-confirm");
    assert_eq!(ui.dialog(), None);
    let after = ui.settings::<SettingsCategoriesSnapshot>().category_types;
    assert_ne!(after, before);
    let changed = after.iter().zip(&before).filter(|(a, b)| a != b).count();
    assert!(changed >= 2, "Food and at least one child changed type");
    assert!(
        after
            .iter()
            .zip(&before)
            .filter(|(a, b)| a != b)
            .all(|(a, _)| a.1 == lib_core::CategoryTypes::Income)
    );
}
