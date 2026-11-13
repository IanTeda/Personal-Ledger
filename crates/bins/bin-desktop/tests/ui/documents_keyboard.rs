//! The Documents page driven by keys alone: landing, the Inbox/Library toggle, the index and list
//! zones, the Library's sort and search, the Inbox's accept, skip and undo, the link picker, `L`,
//! the Add, Import, Edit, Edit facts, Accept all and Delete Dialogs, their refusals, and the palette's
//! `documents` commands. Assertions read state back
//! from `Root`, never pixels.

#![expect(
    clippy::panic,
    reason = "test support: a row that never comes into focus should fail the test loudly"
)]

use super::common;
use lib_database::Domain;

use bin_desktop::navigation::nav::{InputMode, Noun};
use bin_desktop::shell::router::{DocumentsSnapshot, SettingsPayeesSnapshot};
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

/// The page's key is one input: the filing and the Toast saying so commit once together, through
/// the one door.
#[gpui::test]
fn an_accept_is_one_input_committed_once_with_its_toast(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let toasts = ui.documents().toasts.len();

    let before = ui.commits();
    ui.press("y");

    assert_eq!(
        ui.documents().toasts.len(),
        toasts + 1,
        "the accept says so"
    );
    assert_eq!(ui.commits(), before + 1);
}

#[gpui::test]
fn u_with_nothing_to_undo_changes_nothing_and_says_so(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    let before = ui.documents();

    ui.press("u");

    let after = ui.documents();
    assert_eq!(after.status.as_deref(), Some("Nothing to undo."));
    assert_eq!(
        DocumentsSnapshot {
            status: None,
            ..after
        },
        before
    );
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

/// Skip is the session's: it writes nothing, so a restart brings the row back to its place.
#[gpui::test]
fn a_skip_is_gone_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = common::new_ledger(&dir.path().join("ledger.pldb"));
    {
        let storage = bin_desktop::start_on(&config, common::today())
            .storage()
            .expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        ui.press("g f");
        ui.press("i");
        focus_row(&mut ui, STRONG_RECEIPT);
        ui.press("x");
        ui.press("G");
        assert_eq!(
            ui.documents().selected_file.as_deref(),
            Some(STRONG_RECEIPT),
            "the skipped row sinks to the end"
        );
        ui.settle_writes();
        bin_desktop::quit(&writer);
    }

    let storage = bin_desktop::start_on(&config, common::today())
        .storage()
        .expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    ui.press("g f");
    ui.press("i");
    ui.press("G");

    assert_ne!(
        ui.documents().selected_file.as_deref(),
        Some(STRONG_RECEIPT),
        "the skip was never saved"
    );
}

#[gpui::test]
fn a_failed_accept_puts_the_inbox_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Documents]);
    ui.press("g f");
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let stored = ui.documents().inbox_rows;

    ui.press("y");
    assert_eq!(ui.documents().inbox_rows, stored - 1);

    ui.settle_writes();

    assert_eq!(ui.documents().inbox_rows, stored);
    assert!(
        ui.toasts()
            .visible
            .into_iter()
            .any(|toast| toast.kind == "Error")
    );
}

/// The lease ends 115 days out: Upcoming in June, and within the 60-day window once the month has
/// turned to August, with no key pressed between.
#[gpui::test]
fn the_month_turning_moves_a_key_date_into_need_review(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "lease-agreement-12-elm-st.pdf");
    let (_, red) = ui
        .documents()
        .selected_flag
        .expect("the lease has a Key Date");
    assert!(!red, "Upcoming is muted");

    ui.turn_date_to(common::date(2026, 8, 15));

    let page = ui.documents();
    assert_eq!(
        page.selected_file.as_deref(),
        Some("lease-agreement-12-elm-st.pdf")
    );
    let (_, red) = page.selected_flag.expect("the lease has a Key Date");
    assert!(red, "Need Review is red");
}

/// The Inbox's keys are the keymap's: a rebound Accept moves, and the legend and `?` follow it.
#[gpui::test]
fn a_rebound_accept_moves_and_the_cheat_sheet_follows(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(
        app,
        lib_config::KeyBindingConfig::overriding(lib_documents::keymap::ACCEPT, "ctrl+y"),
    );
    ui.press("g f");
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents().inbox_rows;

    ui.press("y");
    assert_eq!(ui.documents().inbox_rows, before, "y no longer accepts");
    ui.press("ctrl-y");
    assert_eq!(ui.documents().inbox_rows, before - 1);

    assert!(
        ui.status_as_shortcuts()
            .iter()
            .any(|(_, chord)| chord == "ctrl+y")
    );
    ui.assert_cheat_sheet_matches_status("Documents keys");
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
fn a_failed_documents_write_puts_the_inbox_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Documents]);
    ui.press("g f");
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let stored = ui.documents().inbox_rows;

    ui.press("l");
    ui.press("enter");
    assert_eq!(
        ui.documents().inbox_rows,
        stored - 1,
        "memory runs ahead of the file until the failure is heard"
    );

    ui.settle_writes();

    assert_eq!(ui.documents().inbox_rows, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to documents, so they are back as last saved"
    }));
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
fn ctrl_n_and_ctrl_p_move_the_pickers_highlight(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("l");
    let start = ui.documents().picker.expect("picker open").selected;

    ui.press("ctrl-n");
    assert_eq!(
        ui.documents().picker.expect("open").selected,
        start + 1,
        "`ctrl-n` steps down like `down`"
    );

    ui.press("ctrl-p");
    assert_eq!(
        ui.documents().picker.expect("open").selected,
        start,
        "`ctrl-p` steps back up like `up`"
    );
    assert_eq!(ui.documents().dialog, Some("picker-link"));
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

    ui.shell.update(ui.cx, |shell, cx| {
        shell.drop_documents(&[dropped.clone(), dir.join("missing.pdf")], cx);
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

#[gpui::test]
fn o_on_a_file_that_is_missing_says_so_instead_of_opening_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let before = ui.documents().toasts.len();

    // The sample Documents name files that are not on this machine; the OS hand-off (an event the
    // test platform cannot service) is only raised for a file that exists.
    ui.press("o");

    let page = ui.documents();
    assert_eq!(
        page.toasts.len(),
        before + 1,
        "a warning toast names the file"
    );
}

#[gpui::test]
fn question_mark_lists_the_documents_keys_under_their_own_heading(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.assert_cheat_sheet_matches_status("Documents keys");
}

#[gpui::test]
fn deleting_a_linked_payee_drops_its_link_and_keeps_the_document(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = common::new_ledger(&dir.path().join("ledger.pldb"));
    let storage = bin_desktop::start_on(&config, common::today())
        .storage()
        .expect("a new Ledger starts");
    let writer = storage.writer.clone();
    let mut ui = Harness::with_storage(app, storage);
    // A new Payee nothing else names, so the file lets it be deleted.
    ui.press("g s");
    ui.press(&["j"; 10].join(" "));
    ui.press("l");
    ui.press("n z o o enter");
    // The Amex statement's one Link, its Account, swapped for the new Payee.
    ui.press("g f");
    ui.press("l");
    ui.press("enter");
    ui.press("l");
    ui.press("z o o");
    ui.press("enter");
    let linked = ui.documents();
    assert_eq!(
        linked.selected_file.as_deref(),
        Some("amex-platinum-statement.pdf")
    );
    assert_eq!(linked.selected_chips, ["zoo"]);

    ui.press("g s");
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some("zoo")
    );
    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeletePayee"));
    ui.press("enter");
    ui.press("z o o enter");
    assert_eq!(ui.dialog(), None, "the Payee is deleted");
    writer.flush().expect("the writer is running");
    ui.settle_writes();
    ui.press("g f");

    let page = ui.documents();
    assert_eq!(
        page.selected_file.as_deref(),
        Some("amex-platinum-statement.pdf")
    );
    assert_eq!(page.selected_links, 0, "the Document stays, now not linked");
    assert!(page.selected_chips.is_empty());
}

/// Types `text` into the open Dialog, a key per character.
fn type_text(ui: &mut Harness<'_>, text: &str) {
    for ch in text.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
}

/// A fresh directory under the system's temp folder for one test's files.
#[expect(
    clippy::expect_used,
    reason = "test support: a temp folder that can't be made should fail the test loudly"
)]
fn scratch(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pl-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir is writable");
    dir
}

#[gpui::test]
fn add_refuses_a_missing_file_an_unsupported_kind_and_one_already_in_the_library(
    app: &mut TestAppContext,
) {
    let mut ui = on_documents(app);
    let dir = scratch("add");
    let receipt = dir.join("receipt.pdf");
    let notes = dir.join("notes.txt");
    std::fs::write(&receipt, b"stub").expect("temp file is writable");
    std::fs::write(&notes, b"stub").expect("temp file is writable");
    let before = ui.documents().library_rows;

    ui.press("a");
    type_text(&mut ui, &dir.join("missing.pdf").display().to_string());
    assert!(
        !ui.dialog_value("error").unwrap_or_default().is_empty(),
        "a path with no file under it is refused"
    );
    ui.press("enter");
    assert_eq!(
        ui.documents().dialog,
        Some("add"),
        "a refusal keeps it open"
    );

    ui.press("escape");
    ui.press("a");
    type_text(&mut ui, &notes.display().to_string());
    assert!(
        !ui.dialog_value("error").unwrap_or_default().is_empty(),
        "a file that is not a PDF, JPG or PNG is refused"
    );
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));

    ui.press("escape");
    ui.press("a");
    type_text(&mut ui, &receipt.display().to_string());
    assert_eq!(
        ui.dialog_value("document_title").as_deref(),
        Some("receipt")
    );
    ui.press("enter");
    let added = ui.documents();
    assert_eq!(added.dialog, None);
    assert_eq!(added.library_rows, before + 1, "the file is added");
    assert_eq!(added.selected_file.as_deref(), Some("receipt.pdf"));

    ui.press("a");
    type_text(&mut ui, &receipt.display().to_string());
    let already = ui.dialog_value("error").unwrap_or_default();
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        already.contains("receipt"),
        "a file already in the Library names its Document: {already}"
    );
    ui.press("enter");
    assert_eq!(
        ui.documents().library_rows,
        before + 1,
        "nothing more lands"
    );
}

#[gpui::test]
fn an_import_that_takes_nothing_stays_open_and_says_why(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let dir = scratch("import");
    let scan = dir.join("scan.pdf");
    std::fs::write(&scan, b"stub").expect("temp file is writable");
    let before = ui.documents().inbox_rows;

    ui.press("I");
    type_text(&mut ui, &dir.join("missing.pdf").display().to_string());
    ui.press("ctrl-enter");
    assert_eq!(ui.documents().dialog, Some("import"), "nothing imported");
    assert!(
        !ui.dialog_value("notes").unwrap_or_default().is_empty(),
        "each refused path says why"
    );

    ui.press("enter");
    type_text(&mut ui, &scan.display().to_string());
    ui.press("ctrl-enter");
    std::fs::remove_dir_all(&dir).ok();

    let page = ui.documents();
    assert_eq!(page.dialog, None, "an import that takes any closes");
    assert_eq!(
        page.inbox_rows,
        before + 1,
        "only the file that exists lands"
    );
    assert_eq!(page.toasts.len(), 1);
}

#[gpui::test]
fn e_edits_the_selected_documents_title(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let file = ui.documents().selected_file;

    ui.press("e");
    assert_eq!(ui.documents().dialog, Some("edit"));
    let title = ui.dialog_value("document_title").unwrap_or_default();
    assert!(!title.is_empty(), "the form opens on the Document's Title");

    type_text(&mut ui, " 2");
    ui.press("enter");

    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert_eq!(page.selected_file, file, "the Document stays selected");
    assert!(
        page.toasts
            .iter()
            .any(|toast| toast.contains(&format!("{title} 2"))),
        "the Toast names the new Title: {:?}",
        page.toasts
    );
}

#[gpui::test]
fn e_in_the_inbox_edits_the_extracted_facts_and_refuses_a_bad_total(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);

    ui.press("e");
    assert_eq!(ui.documents().dialog, Some("facts"));

    // Merchant, then Date, then Total.
    ui.press("tab");
    ui.press("tab");
    type_text(&mut ui, "abc");
    assert!(!ui.dialog_value("error").unwrap_or_default().is_empty());
    ui.press("enter");
    assert_eq!(
        ui.documents().dialog,
        Some("facts"),
        "a bad total is refused"
    );

    for _ in 0..3 {
        ui.press("backspace");
    }
    ui.press("enter");
    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert_eq!(page.toasts.len(), 1, "the save is announced");
}

#[gpui::test]
fn accept_all_files_the_count_it_quotes_and_u_undoes_them_all(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    let before = ui.documents().inbox_rows;

    ui.press("Y");
    let body = ui.dialog_value("body").unwrap_or_default();
    ui.press("enter");
    let filed = ui.documents();
    assert_eq!(filed.dialog, None);
    let count = before - filed.inbox_rows;
    assert!(count > 0, "the sample has strong matches");
    assert!(
        body.contains(&count.to_string()),
        "the confirm quotes the count it files: {body}"
    );

    ui.press("u");
    assert_eq!(ui.documents().inbox_rows, before, "one `u` undoes them all");
}

#[gpui::test]
fn d_in_the_library_confirms_then_deletes_the_document_and_esc_keeps_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let before = ui.documents();
    let file = before.selected_file.clone();

    ui.press("d");
    assert_eq!(ui.documents().dialog, Some("delete"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    let body = ui.dialog_value("body").unwrap_or_default();
    assert!(
        body.contains("The file stays on disk at"),
        "the confirm says the file stays: {body}"
    );
    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(ui.documents().library_rows, before.library_rows);

    ui.press("d");
    ui.press("enter");

    let after = ui.documents();
    assert_eq!(after.dialog, None);
    assert_eq!(after.library_rows, before.library_rows - 1);
    assert_ne!(after.selected_file, file, "focus moves to the next row");
    assert_eq!(
        after.inbox_rows, before.inbox_rows,
        "the Inbox is untouched"
    );
    assert!(
        after
            .toasts
            .iter()
            .any(|toast| toast.contains("The file is still on disk.")),
        "{:?}",
        after.toasts
    );
}

#[gpui::test]
fn d_in_the_inbox_confirms_then_deletes_and_u_has_nothing_to_undo_for_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents();

    ui.press("d");
    assert_eq!(ui.documents().dialog, Some("delete"));
    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(ui.documents().inbox_rows, before.inbox_rows);

    // Accept it, so a filing Undo names it, then delete it from the Library.
    ui.press("y");
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    ui.press("d");
    ui.press("enter");
    assert_eq!(ui.documents().library_rows, before.library_rows);
    ui.press("i");
    ui.press("u");
    assert_eq!(
        ui.documents().status.as_deref(),
        Some("Nothing to undo."),
        "`u` never reports undoing a Document that is gone"
    );
}

#[gpui::test]
fn d_on_an_unfiled_inbox_row_deletes_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("i");
    focus_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents().inbox_rows;

    ui.press("d");
    ui.press("enter");

    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert_eq!(page.inbox_rows, before - 1);
    assert_ne!(page.selected_file.as_deref(), Some(STRONG_RECEIPT));
}

/// `/` reaches the Library's Search through the root Router, inside the key's one input, so it
/// commits once and opens no input of its own.
#[gpui::test]
fn slash_opens_the_search_in_one_input_committed_once(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    let before = ui.commits();
    ui.press("/");

    assert_eq!(mode(&mut ui), InputMode::Search);
    assert_eq!(ui.commits(), before + 1);
}

/// `esc` backs the list out to the index through the root Router, inside the key's one input.
#[gpui::test]
fn esc_on_the_list_is_one_input_committed_once(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    assert!(!ui.documents().index_focused);

    let before = ui.commits();
    ui.press("escape");

    assert!(ui.documents().index_focused, "`esc` backs out to the index");
    assert_eq!(ui.commits(), before + 1);
}
