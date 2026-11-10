//! The Settings Documents page driven by keys: row selection, `J`/`K` reorder and `x` on Other.
//! State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsDocumentTypesSnapshot;
use lib_database::Domain;

use common::Harness;
use gpui::TestAppContext;
use lib_documents::DocumentType;

/// Settings on the Documents page (eleven presses down the index) with focus stepped into it.
fn in_documents(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(&["j"; 11].join(" "));
    assert_eq!(ui.settings_frame().page, "Documents");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui
}

#[gpui::test]
fn the_page_lists_the_nine_seeded_types_ending_with_other(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(document_types_page.document_type_names.len(), 9);
    assert_eq!(
        document_types_page
            .document_type_names
            .last()
            .map(String::as_str),
        Some("Other")
    );
    assert_eq!(
        document_types_page.selected_document_type.as_deref(),
        Some("Receipts")
    );
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    let names = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_names;
    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .selected_document_type
            .as_ref(),
        names.get(1)
    );
    ui.press("shift-g");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .selected_document_type
            .as_ref(),
        names.last()
    );
    ui.press("k");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .selected_document_type
            .as_ref(),
        names.get(7)
    );
    ui.press("g g");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .selected_document_type
            .as_ref(),
        names.first()
    );
}

#[gpui::test]
fn shift_j_and_shift_k_reorder_the_selected_type(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-j");
    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(document_types_page.document_type_names[0], "Statements");
    assert_eq!(document_types_page.document_type_names[1], "Receipts");
    assert_eq!(
        document_types_page.selected_document_type.as_deref(),
        Some("Receipts")
    );
    ui.press("shift-k");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names[0],
        "Receipts"
    );
    // Already first: nothing moves.
    ui.press("shift-k");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names[0],
        "Receipts"
    );
}

#[gpui::test]
fn other_reorders_but_x_on_it_removes_nothing_and_says_why(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-g");
    ui.press("shift-k");
    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(document_types_page.document_type_names[7], "Other");
    assert_eq!(document_types_page.document_type_names.len(), 9);
    ui.press("x");
    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    let page = ui.settings_frame();
    assert_eq!(document_types_page.document_type_names.len(), 9);
    assert_eq!(
        page.status_message.as_deref(),
        Some("Other can't be removed")
    );
}

#[gpui::test]
fn n_opens_the_add_dialog_and_enter_adds_a_type_at_the_end(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddDocumentType"));
    // Nothing typed yet: enter is a no-op.
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddDocumentType"));
    ui.press("l e a s e s");
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(document_types_page.document_type_names.len(), 10);
    assert_eq!(
        document_types_page
            .document_type_names
            .last()
            .map(String::as_str),
        Some("leases")
    );
    assert_eq!(
        document_types_page.selected_document_type.as_deref(),
        Some("leases")
    );
}

#[gpui::test]
fn add_refuses_a_name_another_type_has_ignoring_case(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    ui.press("t a x");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddDocumentType"));
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names
            .len(),
        9
    );
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn edit_renames_keeping_the_row_and_its_files(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditDocumentType"));
    ui.press("backspace backspace backspace");
    ui.press("t a x e s");
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(document_types_page.document_type_names[2], "taxes");
    assert_eq!(document_types_page.document_type_files[2], 31);
    assert_eq!(
        document_types_page.selected_document_type.as_deref(),
        Some("taxes")
    );
}

#[gpui::test]
fn edit_keeps_its_own_name_valid_in_another_case(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("e");
    ui.press("backspace backspace backspace");
    ui.press("T A X");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names[2],
        "TAX"
    );
}

#[gpui::test]
fn x_asks_for_a_destination_and_moves_the_files_to_other(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveDocumentType"));
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(document_types_page.document_type_names.len(), 8);
    assert!(
        !document_types_page
            .document_type_names
            .contains(&"Tax".to_string())
    );
    // Other had 2 files and takes Tax's 31.
    assert_eq!(document_types_page.document_type_files.last(), Some(&33));
}

#[gpui::test]
fn the_destination_can_be_changed_with_the_arrow_keys(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("x");
    // Other is last among the eight options: down wraps to the first, Receipts.
    ui.press("down");
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(document_types_page.document_type_files[0], 186 + 31);
    assert_eq!(document_types_page.document_type_files.last(), Some(&2));
}

#[gpui::test]
fn x_on_a_type_with_no_files_removes_it_with_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    // Add a fresh type (no Filed files), then remove it.
    ui.press("n");
    ui.press("l e a s e s");
    ui.press("enter");

    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveDocumentType"));
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(document_types_page.document_type_names.len(), 9);
    assert_eq!(
        document_types_page.document_type_files.iter().sum::<u32>(),
        392
    );
}

#[gpui::test]
fn escape_cancels_remove_and_changes_nothing(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("x");
    ui.press("escape");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(document_types_page.document_type_names.len(), 9);
}

#[gpui::test]
fn x_on_other_opens_the_notice_and_enter_closes_it(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("shift-g");
    ui.press("x");
    assert_eq!(ui.dialog(), Some("DefaultNotice"));
    ui.press("enter");

    let document_types_page = ui.settings::<SettingsDocumentTypesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(document_types_page.document_type_names.len(), 9);
}

/// Whole-library row count as the Documents surface sees it.
fn library_rows(ui: &mut Harness<'_>) -> usize {
    ui.press("g f");
    ui.documents().library_rows
}

#[gpui::test]
fn removing_a_type_in_settings_keeps_every_document_in_the_library(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    let before = {
        ui.press("g f");
        let rows = ui.documents().library_rows;
        ui.press("g s");
        if !ui.settings_frame().page_focused {
            ui.press("l");
        }
        assert_eq!(ui.settings_frame().page, "Documents");
        rows
    };
    ui.press("j j");
    ui.press("x");
    ui.press("enter");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names
            .len(),
        8
    );

    assert_eq!(library_rows(&mut ui), before);
}

#[gpui::test]
fn a_type_added_in_settings_gets_a_facet_on_the_documents_rail(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("n");
    ui.press("l e a s e s");
    ui.press("enter");
    // The new type is the last row, after the nine seeded ones.
    let added = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_ids
        .last()
        .copied()
        .expect("the added type");
    ui.press("g f");

    ui.click(&format!(
        "documents-rail-Scope(Type({:?}))",
        DocumentType(added)
    ));

    let page = ui.documents();
    assert_eq!(page.scope, format!("type:{added}"));
    assert_eq!(page.library_rows, 0);
}

#[gpui::test]
fn removing_the_type_a_library_is_scoped_to_falls_back_to_all_documents(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("n");
    ui.press("l e a s e s");
    ui.press("enter");
    let added = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_ids
        .last()
        .copied()
        .expect("the added type");
    ui.press("g f");
    ui.click(&format!(
        "documents-rail-Scope(Type({:?}))",
        DocumentType(added)
    ));
    assert_eq!(ui.documents().scope, format!("type:{added}"));

    ui.press("g s");
    if !ui.settings_frame().page_focused {
        ui.press("l");
    }
    // The added type is selected, has no Filed files, and goes on a plain confirm.
    ui.press("x");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.documents().scope, "all");
}

#[gpui::test]
fn the_form_fields_take_tab_the_arrows_and_space(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Name"));
    ui.press("l e a s e s");
    // Remind is dimmed, and skipped, while no date is tracked.
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("TracksDate"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("FinancialYear"));
    ui.press("shift-tab");
    ui.press("right right");
    assert_eq!(ui.dialog_value("tracks_date").as_deref(), Some("Ends"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Remind"));
    ui.press("right right");
    assert_eq!(ui.dialog_value("remind").as_deref(), Some("14 d"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("FinancialYear"));
    ui.press("space");
    assert_eq!(ui.dialog_value("financial_year").as_deref(), Some("true"));
    // A control swallows typing rather than letting it reach the name.
    ui.press("x");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("leases"));
    ui.press("enter");
    assert_eq!(ui.dialog(), None);

    // The added type is selected, and Edit reads back what was saved.
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditDocumentType"));
    assert_eq!(ui.dialog_value("tracks_date").as_deref(), Some("Ends"));
    assert_eq!(ui.dialog_value("remind").as_deref(), Some("14 d"));
    assert_eq!(ui.dialog_value("financial_year").as_deref(), Some("true"));
}

#[gpui::test]
fn clearing_the_tracks_date_clears_remind(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    // Insurance renews and reminds 30 days ahead.
    ui.press("j j j");
    ui.press("e");
    assert_eq!(ui.dialog_value("tracks_date").as_deref(), Some("Renews"));
    assert_eq!(ui.dialog_value("remind").as_deref(), Some("30 d"));

    ui.press("tab");
    ui.press("left");
    assert_eq!(ui.dialog_value("tracks_date").as_deref(), Some("None"));
    assert_eq!(ui.dialog_value("remind").as_deref(), Some("None"));
    ui.press("enter");
    ui.press("e");
    assert_eq!(ui.dialog_value("remind").as_deref(), Some("None"));
}

#[gpui::test]
fn the_edit_dialog_opens_on_the_selected_type_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j j");

    ui.press("e");

    assert_eq!(
        ui.dialog_value("title").as_deref(),
        Some("Edit Document type")
    );
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_name_error_shows_once_the_name_is_edited(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press("n");
    assert_eq!(ui.dialog_value("error").as_deref(), Some(""));
    ui.press("t a x");
    assert_eq!(
        ui.dialog_value("error").as_deref(),
        Some("Another type already has that name.")
    );
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
}

#[gpui::test]
fn a_remove_destination_defaults_to_other(app: &mut TestAppContext) {
    let mut ui = in_documents(app);
    ui.press("j j");

    ui.press("x");

    assert_eq!(ui.dialog_value("destination").as_deref(), Some("Other"));
    assert_eq!(
        ui.dialog_value("message").as_deref(),
        Some("Tax has 31 Filed files. Move them to:")
    );
}

#[gpui::test]
fn a_failed_write_puts_the_types_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Documents]);
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");
    let stored = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_names;

    ui.press("n l e a s e s enter");
    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names
            .len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names,
        stored
    );
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to documents, so they are back as last saved"
    }));
}

/// The Documents and the Document Types share one store, so a failure naming both reads it back
/// once, with one Toast.
#[gpui::test]
fn a_failed_write_naming_both_document_domains_shows_one_toast(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| {
        fakes.fails.writes = vec![Domain::Documents, Domain::DocumentTypes];
    });
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");

    ui.press("n l e a s e s enter");
    ui.settle_writes();

    let errors: Vec<u32> = ui
        .toasts()
        .visible
        .into_iter()
        .filter(|toast| toast.kind == "Error")
        .map(|toast| toast.count)
        .collect();
    assert_eq!(errors, [1], "one Toast, raised once");
}

#[gpui::test]
fn a_failed_reorder_puts_the_order_back(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Documents]);
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");
    let stored = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_names;

    ui.press("shift-j");
    assert_ne!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names,
        stored
    );
    ui.settle_writes();

    assert_eq!(
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names,
        stored
    );
}

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn the_cheat_sheet_shows_a_rebound_key_and_the_key_works(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(
        app,
        common::overriding("documents.move_type_down", "ctrl+j"),
    );
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");
    assert!(
        ui.status_as_shortcuts()
            .iter()
            .any(|(_, keys)| keys == "ctrl+j/K"),
        "the status line shows the rebound key"
    );

    ui.press("ctrl-j");
    let names = ui
        .settings::<SettingsDocumentTypesSnapshot>()
        .document_type_names;
    assert_eq!(names[..2], ["Statements", "Receipts"]);
    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn the_palette_lists_the_page_commands_and_runs_them(app: &mut TestAppContext) {
    let mut ui = in_documents(app);

    ui.press(":");
    ui.press("d o c u m e n t - t y p e s space n e w");
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("AddDocumentType"));
}
