//! The Documents page driven by the mouse alone: the primary and index rails, list rows, the
//! detail pane's LINKED TO names and ×, the header buttons and the Inbox's Accept and Skip.
//! Targets are found with `debug_selector` tags and `debug_bounds`, never coordinates; assertions
//! read state back from `Shell`.

#![expect(
    clippy::panic,
    reason = "test support: a row that never comes into focus should fail the test loudly"
)]

mod common;

use bin_desktop::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

const STRONG_RECEIPT: &str = "IMG_4471.jpg";
const UNREADABLE_SCAN: &str = "scan0012.pdf";

/// Opens Documents by clicking its primary rail row.
fn on_documents(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.click("primary-rail-Documents");
    ui
}

/// Moves the selection to `file` with `j`. Rows below the fold have no on-screen bounds to
/// click, so reaching them is set-up, not the flow under test.
fn focus_row(ui: &mut Harness<'_>, file: &str) {
    for _ in 0..200 {
        if ui.documents().selected_file.as_deref() == Some(file) {
            return;
        }
        ui.press("j");
    }
    unreachable!("never reached `{file}`");
}

/// Clicks Inbox rows until the selection is on `file`, returning that row's index.
fn click_inbox_row(ui: &mut Harness<'_>, file: &str) -> usize {
    for index in 0..ui.documents().inbox_rows {
        ui.click(&format!("documents-inbox-row-{index}"));
        if ui.documents().selected_file.as_deref() == Some(file) {
            return index;
        }
    }
    panic!("no Inbox row shows `{file}`");
}

#[gpui::test]
fn the_primary_rail_opens_documents(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    assert_eq!(ui.noun(), Noun::Documents);
    assert_eq!(ui.documents().mode, "library");
}

#[gpui::test]
fn index_rail_inbox_and_all_switch_the_mode(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("documents-rail-Inbox");
    assert_eq!(ui.documents().mode, "inbox");

    ui.click("documents-rail-Scope(All)");
    let page = ui.documents();
    assert_eq!(page.mode, "library");
    assert_eq!(page.scope, "all");
}

#[gpui::test]
fn index_rail_facets_narrow_the_library(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let all = ui.documents().library_rows;

    ui.click("documents-rail-Scope(Type(Receipt))");

    let page = ui.documents();
    assert_eq!(page.scope, "type:receipts");
    assert_eq!(page.mode, "library");
    assert!(page.library_rows < all, "a type facet narrows the list");
}

#[gpui::test]
fn clicking_a_list_row_selects_it_and_focuses_the_list(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let first = ui.documents().selected_file;
    ui.press("j");
    let second = ui.documents().selected_file;
    assert_ne!(second, first);
    ui.press("k");
    ui.press("h");
    assert!(ui.documents().index_focused);

    ui.click("documents-row-1");

    let page = ui.documents();
    assert_eq!(page.selected_file, second);
    assert!(!page.index_focused, "a row click puts focus on the list");
}

#[gpui::test]
fn a_single_link_name_navigates_to_its_target(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    // The first Library row, the Amex statement, links only to its Account.
    assert_eq!(
        ui.documents().selected_file.as_deref(),
        Some("amex-platinum-statement.pdf")
    );

    ui.click("documents-link-0");

    assert_eq!(ui.noun(), Noun::Settings, "an Account lives in Settings");
}

#[gpui::test]
fn each_link_kind_navigates_to_its_own_target(app: &mut TestAppContext) {
    // The AAMI policy links, in order, to a Payee, a Bill Plan and an Inventory item.
    let targets = [Noun::Settings, Noun::Bills, Noun::Inventory];
    for (index, target) in targets.into_iter().enumerate() {
        let mut ui = on_documents(app);
        focus_row(&mut ui, "aami-car-policy.pdf");
        assert_eq!(ui.documents().selected_links, 3);

        ui.click(&format!("documents-link-{index}"));

        assert_eq!(ui.noun(), target, "link {index}");
    }
}

#[gpui::test]
fn a_transaction_link_opens_transactions(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "receipt-camera-house.pdf");

    // Linked to an Inventory item first, then the purchase Transaction.
    ui.click("documents-link-1");

    assert_eq!(ui.noun(), Noun::Transactions);
}

#[gpui::test]
fn the_x_unlinks_without_following_the_link(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    assert_eq!(ui.documents().selected_links, 1);

    ui.click("documents-unlink-0");

    let page = ui.documents();
    assert_eq!(page.selected_links, 0);
    assert_eq!(ui.noun(), Noun::Documents, "the × must not follow the link");
    assert!(!page.toasts.is_empty(), "unlinking says so with a Toast");
}

#[gpui::test]
fn add_and_import_buttons_open_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("documents-add");
    assert_eq!(ui.documents().dialog, Some("add"));
    ui.press("escape");
    assert_eq!(ui.documents().dialog, None);

    ui.click("documents-import");
    assert_eq!(ui.documents().dialog, Some("import"));
}

#[gpui::test]
fn watched_folder_button_raises_a_toast(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    assert!(ui.documents().toasts.is_empty());

    ui.click("documents-watched-folder");

    assert_eq!(ui.documents().toasts.len(), 1);
}

#[gpui::test]
fn accept_all_button_opens_its_confirmation(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");

    ui.click("documents-accept-all");

    assert_eq!(ui.documents().dialog, Some("accept-all"));
}

#[gpui::test]
fn inbox_row_accept_files_the_document(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    let before = ui.documents().inbox_rows;
    let index = click_inbox_row(&mut ui, STRONG_RECEIPT);

    ui.click(&format!("documents-inbox-accept-{index}"));

    let page = ui.documents();
    assert_eq!(page.inbox_rows, before - 1);
    assert_ne!(page.selected_file.as_deref(), Some(STRONG_RECEIPT));
}

#[gpui::test]
fn inbox_detail_skip_keeps_an_unreadable_scan_in_the_inbox(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    // Nothing can be read from the scan, so its detail offers Skip rather than Link elsewhere.
    click_inbox_row(&mut ui, UNREADABLE_SCAN);
    let before = ui.documents();

    ui.click("documents-inbox-secondary");

    let after = ui.documents();
    assert_eq!(after.inbox_rows, before.inbox_rows, "a skip never files");
    assert_ne!(after.selected_file.as_deref(), Some(UNREADABLE_SCAN));
}

#[gpui::test]
fn inbox_detail_link_elsewhere_opens_the_picker(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    click_inbox_row(&mut ui, STRONG_RECEIPT);

    ui.click("documents-inbox-secondary");

    assert_eq!(ui.documents().dialog, Some("picker-file"));
}

#[gpui::test]
fn inbox_detail_primary_files_a_strong_match(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    let before = ui.documents().inbox_rows;
    click_inbox_row(&mut ui, STRONG_RECEIPT);

    ui.click("documents-inbox-primary");

    let page = ui.documents();
    assert_eq!(page.inbox_rows, before - 1, "the primary button files it");
    assert_ne!(page.selected_file.as_deref(), Some(STRONG_RECEIPT));
}
