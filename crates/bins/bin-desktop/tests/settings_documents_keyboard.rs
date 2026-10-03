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
fn n_opens_the_add_dialog_and_enter_adds_a_type_at_the_end(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    assert_eq!(ui.settings().document_type_dialog.as_deref(), Some("add"));
    // Nothing typed yet: enter is a no-op.
    ui.press("enter");
    assert_eq!(ui.settings().document_type_dialog.as_deref(), Some("add"));
    ui.press("l e a s e s");
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 10);
    assert_eq!(
        page.document_type_names.last().map(String::as_str),
        Some("leases")
    );
    assert_eq!(page.selected_document_type.as_deref(), Some("leases"));
}

#[gpui::test]
fn add_refuses_a_name_another_type_has_ignoring_case(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    ui.press("t a x");
    ui.press("enter");
    assert_eq!(ui.settings().document_type_dialog.as_deref(), Some("add"));
    assert_eq!(ui.settings().document_type_names.len(), 9);
    ui.press("escape");
    assert_eq!(ui.settings().document_type_dialog, None);
}

#[gpui::test]
fn edit_renames_keeping_the_row_and_its_files(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("e");
    assert_eq!(ui.settings().document_type_dialog.as_deref(), Some("edit"));
    ui.press("backspace backspace backspace");
    ui.press("t a x e s");
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names[2], "taxes");
    assert_eq!(page.document_type_files[2], 31);
    assert_eq!(page.selected_document_type.as_deref(), Some("taxes"));
}

#[gpui::test]
fn edit_keeps_its_own_name_valid_in_another_case(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("e");
    ui.press("backspace backspace backspace");
    ui.press("T A X");
    ui.press("enter");

    assert_eq!(ui.settings().document_type_dialog, None);
    assert_eq!(ui.settings().document_type_names[2], "TAX");
}

#[gpui::test]
fn x_asks_for_a_destination_and_moves_the_files_to_other(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("x");
    assert_eq!(
        ui.settings().document_type_dialog.as_deref(),
        Some("remove")
    );
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 8);
    assert!(!page.document_type_names.contains(&"Tax".to_string()));
    // Other had 2 files and takes Tax's 31.
    assert_eq!(page.document_type_files.last(), Some(&33));
}

#[gpui::test]
fn the_destination_can_be_changed_with_the_arrow_keys(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("x");
    // Other is last among the eight options: down wraps to the first, Receipts.
    ui.press("down");
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_files[0], 186 + 31);
    assert_eq!(page.document_type_files.last(), Some(&2));
}

#[gpui::test]
fn x_on_a_type_with_no_files_removes_it_with_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    // Add a fresh type (no Filed files), then remove it.
    ui.press("n");
    ui.press("l e a s e s");
    ui.press("enter");

    ui.press("x");
    assert_eq!(
        ui.settings().document_type_dialog.as_deref(),
        Some("remove")
    );
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_names.len(), 9);
    assert_eq!(page.document_type_files.iter().sum::<u32>(), 392);
}

#[gpui::test]
fn escape_cancels_remove_and_changes_nothing(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("x");
    ui.press("escape");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 9);
}

#[gpui::test]
fn x_on_other_opens_the_notice_and_enter_closes_it(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-g");
    ui.press("x");
    assert_eq!(
        ui.settings().document_type_dialog.as_deref(),
        Some("notice")
    );
    ui.press("enter");

    let page = ui.settings();
    assert_eq!(page.document_type_dialog, None);
    assert_eq!(page.document_type_names.len(), 9);
}
