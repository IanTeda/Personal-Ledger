//! The Documents page driven by keys alone: landing, the Inbox/Library toggle, the index and list
//! zones, the Library's sort and search, the Inbox's accept, skip and undo, the link picker, `L`,
//! the Add and Import dialogs and the palette's `documents` commands. Assertions read state back
//! from `Shell`, never pixels.

#![expect(
    clippy::panic,
    reason = "test support: a row that never comes into focus should fail the test loudly"
)]

mod common;

use bin_desktop::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

const STRONG_RECEIPT: &str = "IMG_4471.jpg";

/// Opens Documents with `g f`.
fn on_documents(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g f");
    ui
}

/// Moves the list selection down with `j` until it is on `file`.
fn focus_row(ui: &mut Harness<'_>, file: &str) {
    for _ in 0..200 {
        if ui.documents().selected_file.as_deref() == Some(file) {
            return;
        }
        ui.press("j");
    }
    panic!("never reached `{file}`");
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn g_f_lands_on_the_library_list(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    let page = ui.documents();
    assert_eq!(ui.noun(), Noun::Documents);
    assert_eq!(page.mode, "library");
    assert_eq!(page.scope, "all");
    assert!(!page.index_focused, "focus starts on the list");
    assert_eq!(page.dialog, None);
}

#[gpui::test]
fn i_toggles_inbox_and_library(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("i");
    assert_eq!(ui.documents().mode, "inbox");

    ui.press("i");
    assert_eq!(ui.documents().mode, "library");
}

#[gpui::test]
fn zone_moves_between_list_and_index(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("h");
    assert!(ui.documents().index_focused, "`h` goes to the index");

    ui.press("j");
    assert_ne!(
        ui.documents().scope,
        "all",
        "`j` in the index moves the scope"
    );
    assert_eq!(ui.documents().mode, "library");

    ui.press("enter");
    assert!(!ui.documents().index_focused, "`enter` returns to the list");

    ui.press("escape");
    assert!(ui.documents().index_focused, "`esc` backs out to the index");

    ui.press("l");
    assert!(!ui.documents().index_focused, "`l` enters the list");
}

#[gpui::test]
fn index_k_reaches_the_inbox_row_and_shift_g_the_last_year(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("h");

    ui.press("k");
    assert_eq!(ui.documents().mode, "inbox", "the Inbox row sits above All");

    ui.press("G");
    let page = ui.documents();
    assert_eq!(page.mode, "library");
    assert!(
        page.scope.starts_with("fy:before:"),
        "the rail ends on the oldest year facet"
    );
}

#[gpui::test]
fn list_j_and_k_move_the_selection(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let first = ui.documents().selected_file;

    ui.press("j");
    assert_ne!(ui.documents().selected_file, first);

    ui.press("k");
    assert_eq!(ui.documents().selected_file, first);
}

#[gpui::test]
fn s_toggles_the_sort_in_the_library_only(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("s");
    assert_eq!(ui.documents().sort, "expiring");
    ui.press("s");
    assert_eq!(ui.documents().sort, "newest");

    ui.press("i");
    ui.press("s");
    assert_eq!(
        ui.documents().sort,
        "newest",
        "`s` does nothing in the Inbox"
    );
}

#[gpui::test]
fn slash_searches_the_library(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let all = ui.documents().library_rows;

    ui.press("/");
    assert_eq!(mode(&mut ui), InputMode::Search);

    ui.press("a m e x");
    let narrowed = ui.documents();
    assert_eq!(narrowed.query, "amex");
    assert!(narrowed.library_rows < all, "typing narrows live");

    ui.press("enter");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.documents().query, "amex", "`enter` keeps the query");

    ui.press("escape");
    assert_eq!(ui.documents().query, "", "`esc` clears a committed query");
    assert_eq!(ui.documents().library_rows, all);
}

#[gpui::test]
fn esc_in_search_clears_the_query(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("/");
    ui.press("a m e x");
    ui.press("escape");

    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.documents().query, "");
}

#[gpui::test]
fn slash_is_inert_in_the_inbox(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");

    ui.press("/");

    assert_ne!(mode(&mut ui), InputMode::Search);
}

#[gpui::test]
fn y_accepts_the_focused_strong_match_and_u_undoes_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents().inbox_rows;

    ui.press("y");
    let accepted = ui.documents();
    assert_eq!(
        accepted.inbox_rows,
        before - 1,
        "the filed row leaves the Inbox"
    );
    assert_ne!(accepted.selected_file.as_deref(), Some(STRONG_RECEIPT));

    ui.press("u");
    let undone = ui.documents();
    assert_eq!(undone.inbox_rows, before);
    assert_eq!(undone.selected_file.as_deref(), Some(STRONG_RECEIPT));
    assert!(!undone.toasts.is_empty(), "undo says so with a Toast");
}

#[gpui::test]
fn u_with_nothing_to_undo_changes_nothing(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    let before = ui.documents();

    ui.press("u");

    assert_eq!(ui.documents(), before);
}

#[gpui::test]
fn x_skips_the_focused_row(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);

    ui.press("x");

    let page = ui.documents();
    assert_eq!(page.inbox_rows, 7, "a skipped Document stays in the Inbox");
    assert_ne!(
        page.selected_file.as_deref(),
        Some(STRONG_RECEIPT),
        "it sinks to the end"
    );
}

#[gpui::test]
fn y_and_x_do_nothing_in_the_library(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let before = ui.documents();

    ui.press("x");
    ui.press("u");

    assert_eq!(ui.documents(), before);
}

#[gpui::test]
fn shift_y_opens_accept_all_and_esc_cancels(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");

    ui.press("Y");
    assert_eq!(ui.documents().dialog, Some("accept-all"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);

    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.documents().inbox_rows, 7, "cancelling files nothing");
}

#[gpui::test]
fn l_opens_the_link_picker_and_esc_closes_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("l");
    assert_eq!(ui.documents().dialog, Some("picker-link"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);

    ui.press("tab");
    assert_eq!(
        ui.documents().dialog,
        Some("picker-link"),
        "`tab` cycles kinds in place"
    );

    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn enter_in_the_link_picker_toggles_a_link(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("l");
    ui.press("enter");

    let page = ui.documents();
    assert_eq!(page.dialog, None, "picking closes the picker");
    assert_eq!(page.toasts.len(), 1, "the change is announced with a Toast");
}

#[gpui::test]
fn the_link_picker_pins_a_current_link_and_enter_removes_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    // The Amex statement links only to its Account, which the picker pins first.
    let before = ui.documents().selected_links;
    assert_eq!(before, 1);

    ui.press("l");
    ui.press("enter");

    let page = ui.documents();
    assert_eq!(page.selected_links, 0, "enter on a pinned link removes it");
    assert_eq!(
        page.selected_file.as_deref(),
        Some("amex-platinum-statement.pdf"),
        "a Document stays Filed with no links"
    );

    ui.press("l");
    ui.press("down");
    ui.press("enter");
    assert_eq!(ui.documents().selected_links, 1, "any other row adds one");
}

#[gpui::test]
fn picking_in_the_inbox_files_the_document_and_u_undoes_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents().inbox_rows;

    ui.press("l");
    ui.press("enter");
    let filed = ui.documents();
    assert_eq!(filed.dialog, None);
    assert_eq!(filed.inbox_rows, before - 1, "picking files the Document");

    ui.press("u");
    let undone = ui.documents();
    assert_eq!(undone.inbox_rows, before, "one `u` undoes the filing");
    assert_eq!(undone.selected_file.as_deref(), Some(STRONG_RECEIPT));
}

#[gpui::test]
fn l_in_the_inbox_opens_the_filing_picker(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");

    ui.press("l");

    assert_eq!(ui.documents().dialog, Some("picker-file"));
}

#[gpui::test]
fn shift_l_follows_a_single_link(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    // The first Library row, the Amex statement, links only to its Account.
    assert_eq!(
        ui.documents().selected_file.as_deref(),
        Some("amex-platinum-statement.pdf")
    );

    ui.press("L");

    assert_ne!(
        ui.noun(),
        Noun::Documents,
        "`L` leaves for the linked record"
    );
    assert_eq!(ui.documents().dialog, None);
}

#[gpui::test]
fn shift_l_lists_several_links(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "aami-car-policy.pdf");

    ui.press("L");

    assert_eq!(ui.noun(), Noun::Documents);
    assert_eq!(ui.documents().dialog, Some("picker-follow"));
}

#[gpui::test]
fn shift_l_in_the_inbox_follows_the_suggested_link(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);

    ui.press("L");

    assert_eq!(ui.noun(), Noun::Transactions);
}

#[gpui::test]
fn a_opens_the_add_dialog_in_the_library(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("a");

    assert_eq!(ui.documents().dialog, Some("add"));
    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);
}

#[gpui::test]
fn a_does_nothing_in_the_inbox(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");

    ui.press("a");

    assert_eq!(ui.documents().dialog, None);
    assert_ne!(
        mode(&mut ui),
        InputMode::Insert,
        "`a` must not fall through to Insert"
    );
}

#[gpui::test]
fn shift_i_opens_the_import_dialog(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.press("I");

    assert_eq!(ui.documents().dialog, Some("import"));
}

#[gpui::test]
fn palette_documents_inbox_and_library(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(": d o c u m e n t s space i n b o x enter");
    assert_eq!(ui.noun(), Noun::Documents);
    assert_eq!(ui.documents().mode, "inbox");

    ui.press(": d o c u m e n t s space l i b r a r y enter");
    assert_eq!(ui.documents().mode, "library");
}

#[gpui::test]
fn palette_documents_add_import_and_accept_all(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(": d o c u m e n t s space a d d enter");
    assert_eq!(ui.documents().dialog, Some("add"));
    ui.press("escape");

    ui.press(": d o c u m e n t s space i m p o r t enter");
    assert_eq!(ui.documents().dialog, Some("import"));
    ui.press("escape");

    ui.press(": d o c u m e n t s space a c c e p t - a l l enter");
    assert_eq!(ui.documents().dialog, Some("accept-all"));
}

#[gpui::test]
fn the_picker_cycles_kinds_and_filters_by_typing(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("l");
    let all = ui.documents().picker.expect("picker open");
    assert!(all.rows.len() > 1);
    assert!(all.rows[0].2, "the current Link is pinned first and ticked");

    ui.press("tab");
    let txns = ui.documents().picker.expect("picker open");
    assert_ne!(txns.kind, all.kind, "`tab` moves the kind filter");
    assert!(txns.rows.iter().all(|row| row.0 == txns.rows[0].0 || row.2));

    ui.press("tab tab tab tab tab");
    assert_eq!(ui.documents().picker.expect("open").kind, all.kind);

    ui.press("z z z q x");
    let none = ui.documents().picker.expect("open");
    assert_eq!(none.query, "zzzqx");
    assert!(
        none.rows.iter().all(|row| row.2),
        "only pinned rows survive"
    );

    ui.press("backspace");
    assert_eq!(ui.documents().picker.expect("open").query, "zzzq");
}

#[gpui::test]
fn the_inbox_picker_pins_file_without_link_and_draws_its_rows(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    ui.press("l");

    let picker = ui.documents().picker.expect("picker open");
    assert!(!picker.rows[0].3, "File without link is the first row");
    assert!(
        picker.doc_type_chosen,
        "the type is prefilled from the facts"
    );
    assert_eq!(picker.selected, 0);

    ui.click("documents-picker-row-0");
    assert_eq!(ui.documents().dialog, None, "a click on a row picks it");
}

#[gpui::test]
fn files_dropped_on_the_window_land_in_the_inbox(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let before = ui.documents().inbox_rows;
    let dir = std::env::temp_dir().join(format!("pl-drop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir is writable");
    let dropped = dir.join("Coles-Receipt.jpg");
    std::fs::write(&dropped, b"stub").expect("temp file is writable");

    ui.shell.update(ui.cx, |shell, _| {
        shell.drop_documents(&[dropped.clone(), dir.join("missing.pdf")]);
    });
    ui.cx.run_until_parked();
    std::fs::remove_dir_all(&dir).ok();

    let page = ui.documents();
    assert_eq!(
        page.inbox_rows,
        before + 1,
        "only the file that exists lands"
    );
    assert_eq!(page.mode, "library", "a drop does not move the page");
    assert_eq!(page.toasts.len(), 1);
}
