//! The Settings Documents page driven by clicks: row selection and the row actions. Only elements
//! tagged with `debug_selector` are clicked.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_documents(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Documents");
    assert_eq!(ui.settings().page, "Documents");
    ui
}

#[gpui::test]
fn clicking_a_row_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-row-4");
    let page = ui.settings();
    assert_eq!(page.selected_document_type.as_deref(), Some("Insurance"));
    assert!(page.page_focused);
}

#[gpui::test]
fn the_edit_action_selects_its_row(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-edit-6");
    assert_eq!(
        ui.settings().selected_document_type.as_deref(),
        Some("Contracts")
    );
}

#[gpui::test]
fn the_remove_action_selects_its_row_and_other_has_none(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-remove-3");
    assert_eq!(ui.settings().selected_document_type.as_deref(), Some("Tax"));
    // Other (id 9) is drawn with edit but no remove button.
    ui.click("settings-documents-edit-9");
}

#[gpui::test]
fn the_add_button_moves_focus_into_the_page(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-add");
    assert!(ui.settings().page_focused);
}
