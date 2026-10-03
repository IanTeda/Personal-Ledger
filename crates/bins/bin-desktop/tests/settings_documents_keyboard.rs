//! The Settings Documents page driven by keys: row selection, `J`/`K` reorder and `x` on Other.
//! State is read back from `Shell`.

mod common;

use common::Harness;
use gpui::TestAppContext;

/// Settings on the Documents page (eight presses down the index) with focus stepped into it.
fn in_documents(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j j j j j j j j");
    assert_eq!(ui.settings().page, "Documents");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn the_page_lists_the_nine_seeded_types_ending_with_other(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    let page = ui.settings();
    assert_eq!(page.document_type_names.len(), 9);
    assert_eq!(
        page.document_type_names.last().map(String::as_str),
        Some("Other")
    );
    assert_eq!(page.selected_document_type.as_deref(), Some("Receipts"));
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    let names = ui.settings().document_type_names;
    ui.press("j");
    assert_eq!(ui.settings().selected_document_type.as_ref(), names.get(1));
    ui.press("shift-g");
    assert_eq!(ui.settings().selected_document_type.as_ref(), names.last());
    ui.press("k");
    assert_eq!(ui.settings().selected_document_type.as_ref(), names.get(7));
    ui.press("g g");
    assert_eq!(ui.settings().selected_document_type.as_ref(), names.first());
}

#[gpui::test]
fn shift_j_and_shift_k_reorder_the_selected_type(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-j");
    let page = ui.settings();
    assert_eq!(page.document_type_names[0], "Statements");
    assert_eq!(page.document_type_names[1], "Receipts");
    assert_eq!(page.selected_document_type.as_deref(), Some("Receipts"));
    ui.press("shift-k");
    assert_eq!(ui.settings().document_type_names[0], "Receipts");
    // Already first: nothing moves.
    ui.press("shift-k");
    assert_eq!(ui.settings().document_type_names[0], "Receipts");
}

#[gpui::test]
fn other_reorders_but_x_on_it_removes_nothing_and_says_why(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-g");
    ui.press("shift-k");
    let page = ui.settings();
    assert_eq!(page.document_type_names[7], "Other");
    assert_eq!(page.document_type_names.len(), 9);
    ui.press("x");
    let page = ui.settings();
    assert_eq!(page.document_type_names.len(), 9);
    assert_eq!(
        page.status_message.as_deref(),
        Some("Other can't be removed")
    );
}

#[gpui::test]
fn n_and_e_change_nothing_until_the_dialogs_exist(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("n");
    ui.press("e");

    let page = ui.settings();
    assert_eq!(page.document_type_names.len(), 9);
    assert_eq!(page.selected_document_type.as_deref(), Some("Tax"));
    assert!(page.page_focused);
}
