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
fn the_edit_action_opens_the_edit_dialog_and_save_applies_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-edit-6");
    let page = ui.settings();
    assert_eq!(page.selected_document_type.as_deref(), Some("Contracts"));
    assert_eq!(page.document_type_dialog.as_deref(), Some("edit"));
    // Tracks date None (index 0) makes Contracts track nothing; Save applies it.
    ui.click("document-type-tracks-0");
    ui.click("edit-document-type-confirm");
    assert_eq!(ui.settings().document_type_dialog, None);
}

#[gpui::test]
fn the_remove_action_opens_the_dialog_and_other_has_no_button(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    // Other (id 9) is drawn with edit but no remove button; a normal row has both.
    assert!(ui.cx.debug_bounds("settings-documents-edit-9").is_some());
    assert!(ui.cx.debug_bounds("settings-documents-remove-9").is_none());
    assert!(ui.cx.debug_bounds("settings-documents-remove-3").is_some());

    ui.click("settings-documents-remove-3");
    let page = ui.settings();
    assert_eq!(page.selected_document_type.as_deref(), Some("Tax"));
    assert_eq!(page.document_type_dialog.as_deref(), Some("remove"));
    ui.click("remove-document-type-confirm");
    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 8);
    assert_eq!(page.document_type_files.last(), Some(&33));
}

#[gpui::test]
fn the_destination_dropdown_picks_with_a_click(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-remove-3");
    ui.click("document-type-destination");
    // Other is highlighted, so the capped list scrolls to show the last six options: the third,
    // Insurance, is its first row.
    ui.click("document-type-destination-option-2");
    ui.click("remove-document-type-confirm");

    let page = ui.settings();
    assert_eq!(page.document_type_files[2], 18 + 31);
    assert_eq!(page.document_type_files.last(), Some(&2));
}

#[gpui::test]
fn cancel_closes_the_remove_dialog_without_removing(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-remove-3");
    ui.click("remove-document-type-cancel");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 9);
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_the_controls_click(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-add");
    let page = ui.settings();
    assert!(page.page_focused);
    assert_eq!(page.document_type_dialog.as_deref(), Some("add"));
    // Name is empty, so the confirm does nothing.
    ui.click("add-document-type-confirm");
    assert_eq!(ui.settings().document_type_dialog.as_deref(), Some("add"));
    ui.click("document-type-name");
    ui.press("l e a s e s");
    ui.click("document-type-tracks-2");
    ui.click("document-type-remind-3");
    ui.click("document-type-financial-year");
    ui.click("add-document-type-confirm");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(
        page.document_type_names.last().map(String::as_str),
        Some("leases")
    );
}

#[gpui::test]
fn cancel_closes_the_add_dialog(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("settings-documents-add");
    ui.click("add-document-type-cancel");

    assert_eq!(ui.settings().document_type_dialog, None);
    assert_eq!(ui.settings().document_type_names.len(), 9);
}
