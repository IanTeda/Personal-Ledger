//! The Documents page driven by the mouse alone: the primary and index rails, list rows, the
//! detail pane's LINKED TO names and ×, the header buttons, the Inbox's Accept and Skip, and the
//! Dialogs' fields, buttons and link picker rows.
//! Targets are found with `debug_selector` tags and `debug_bounds`, never coordinates; assertions
//! read state back from `Root`.

#![expect(
    clippy::panic,
    reason = "test support: a row that never comes into focus should fail the test loudly"
)]

use super::common;
use bin_desktop::shell::router::{SettingsAccountsSnapshot, SettingsPayeesSnapshot};

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;
use lib_documents::DocumentType;

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

    ui.click(&format!(
        "documents-rail-Scope(Type({:?}))",
        DocumentType::RECEIPTS
    ));

    let page = ui.documents();
    assert_eq!(page.scope, format!("type:{}", DocumentType::RECEIPTS.0));
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
fn an_account_link_selects_the_account_on_its_settings_page(app: &mut TestAppContext) {
    let mut ui = on_documents(app);

    ui.click("documents-link-0");

    assert_eq!(ui.settings_frame().page, "Accounts");
    assert!(
        ui.settings::<SettingsAccountsSnapshot>()
            .selected_account
            .is_some_and(|name| name.contains("Amex")),
        "the Amex statement's Account is selected"
    );
}

#[gpui::test]
fn a_payee_link_selects_the_payee_on_its_settings_page(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "aami-car-policy.pdf");

    ui.click("documents-link-0");

    assert_eq!(ui.settings_frame().page, "Payees");
    assert!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .is_some_and(|name| name.contains("AAMI")),
        "the policy's Payee is selected"
    );
}

#[gpui::test]
fn each_link_kind_navigates_to_its_own_target(app: &mut TestAppContext) {
    // The AAMI policy links, in order, to a Payee, a Bill Plan and an Inventory item.
    // Walked to once: Documents keeps its selection while away, so each return lands on the policy.
    let targets = [Noun::Settings, Noun::Bills, Noun::Inventory];
    let mut ui = on_documents(app);
    focus_row(&mut ui, "aami-car-policy.pdf");
    for (index, target) in targets.into_iter().enumerate() {
        ui.click("primary-rail-Documents");
        let page = ui.documents();
        assert_eq!(page.selected_file.as_deref(), Some("aami-car-policy.pdf"));
        assert_eq!(page.selected_links, 3);

        ui.click(&format!("documents-link-{index}"));

        assert_eq!(ui.noun(), target, "link {index}");
    }
}

#[gpui::test]
fn a_transaction_link_opens_transactions(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "receipt-camera-house.pdf");

    // Linked to an Inventory item first, then the purchase Transaction.
    let before = ui.commits();
    ui.click("documents-link-1");

    assert_eq!(ui.noun(), Noun::Transactions);
    let page = ui.transactions();
    assert!(page.visible > 0, "the linked Transaction is visible");
    assert!(
        !page.payees[page.selected].is_empty(),
        "the linked Transaction is the one selected"
    );
    assert_eq!(ui.commits(), before + 1, "the click is one input");
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

/// Types `text` into the open Dialog, a key per character: what the mouse can't do.
fn type_text(ui: &mut Harness<'_>, text: &str) {
    for ch in text.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
}

#[gpui::test]
fn clicking_a_pinned_link_picker_row_removes_the_link(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    // The Amex statement links only to its Account, which the picker pins first.
    assert_eq!(ui.documents().selected_links, 1);
    ui.press("l");

    ui.click("documents-picker-row-0");

    let page = ui.documents();
    assert_eq!(page.dialog, None, "a click picks and closes");
    assert_eq!(page.selected_links, 0);
    assert!(!page.toasts.is_empty(), "unlinking says so with a Toast");
}

#[gpui::test]
fn clicking_a_follow_picker_row_goes_to_that_record(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    focus_row(&mut ui, "aami-car-policy.pdf");
    ui.press("L");
    assert_eq!(ui.documents().dialog, Some("picker-follow"));

    ui.click("documents-picker-row-0");

    assert_eq!(ui.documents().dialog, None);
    assert_ne!(
        ui.noun(),
        Noun::Documents,
        "the click leaves for the record"
    );
}

#[gpui::test]
fn clicking_a_filing_picker_row_files_the_document_to_it(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    let before = ui.documents().inbox_rows;
    click_inbox_row(&mut ui, STRONG_RECEIPT);
    ui.click("documents-inbox-secondary");
    let picker = ui.documents().picker.expect("the filing picker is open");
    assert!(
        picker.rows[1].3,
        "the row under File without link is a record"
    );

    ui.click("documents-picker-row-1");

    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert_eq!(page.inbox_rows, before - 1, "the Document is filed");
}

#[gpui::test]
fn accept_all_cancel_files_nothing_and_its_confirm_files_the_strong_matches(
    app: &mut TestAppContext,
) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    let before = ui.documents().inbox_rows;

    ui.click("documents-accept-all");
    ui.click("accept-all-cancel");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(
        ui.documents().inbox_rows,
        before,
        "cancelling files nothing"
    );

    ui.click("documents-accept-all");
    ui.click("accept-all-confirm");
    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert!(page.inbox_rows < before, "the strong matches are filed");
}

#[gpui::test]
fn add_and_import_confirm_buttons_refuse_a_missing_file(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    let missing = std::env::temp_dir().join(format!("pl-none-{}.pdf", std::process::id()));

    ui.click("documents-add");
    ui.click("document-path");
    type_text(&mut ui, &missing.display().to_string());
    ui.click("add-document-confirm");
    assert_eq!(
        ui.documents().dialog,
        Some("add"),
        "a refusal keeps it open"
    );
    assert!(!ui.dialog_value("error").unwrap_or_default().is_empty());
    ui.click("add-document-cancel");
    assert_eq!(ui.documents().dialog, None);

    ui.click("documents-import");
    type_text(&mut ui, &missing.display().to_string());
    ui.click("import-documents-confirm");
    assert_eq!(ui.documents().dialog, Some("import"), "nothing imported");
    assert!(!ui.dialog_value("notes").unwrap_or_default().is_empty());
}

#[gpui::test]
fn the_edit_dialogs_fields_and_save_take_clicks(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.press("e");
    let title = ui.dialog_value("document_title").unwrap_or_default();

    ui.click("document-date");
    ui.click("document-title");
    type_text(&mut ui, " 2");
    ui.click("edit-document-confirm");

    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert!(
        page.toasts
            .iter()
            .any(|toast| toast.contains(&format!("{title} 2"))),
        "the click on Title focused it before the typing: {:?}",
        page.toasts
    );
}

#[gpui::test]
fn the_facts_dialogs_save_is_disabled_by_a_bad_total(app: &mut TestAppContext) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    click_inbox_row(&mut ui, STRONG_RECEIPT);
    ui.press("e");
    assert_eq!(ui.documents().dialog, Some("facts"));

    ui.click("facts-total");
    type_text(&mut ui, "abc");
    ui.click("facts-confirm");
    assert_eq!(
        ui.documents().dialog,
        Some("facts"),
        "a bad total is refused"
    );

    ui.press("backspace backspace backspace");
    ui.click("facts-confirm");
    assert_eq!(ui.documents().dialog, None);
}

#[gpui::test]
fn the_library_detail_deletes_after_a_confirm_and_cancel_keeps_the_document(
    app: &mut TestAppContext,
) {
    let mut ui = on_documents(app);
    let before = ui.documents();

    ui.click("documents-delete");
    assert_eq!(ui.documents().dialog, Some("delete"));
    ui.click("delete-document-cancel");
    assert_eq!(ui.documents().dialog, None);
    assert_eq!(ui.documents().library_rows, before.library_rows);

    ui.click("documents-delete");
    ui.click("delete-document-confirm");

    let after = ui.documents();
    assert_eq!(after.dialog, None);
    assert_eq!(after.library_rows, before.library_rows - 1);
    assert_ne!(after.selected_file, before.selected_file);
}

#[gpui::test]
fn the_inbox_detail_deletes_after_a_confirm_and_cancel_keeps_the_document(
    app: &mut TestAppContext,
) {
    let mut ui = on_documents(app);
    ui.click("documents-rail-Inbox");
    click_inbox_row(&mut ui, STRONG_RECEIPT);
    let before = ui.documents().inbox_rows;

    ui.click("documents-inbox-delete");
    assert_eq!(ui.documents().dialog, Some("delete"));
    ui.click("delete-document-cancel");
    assert_eq!(ui.documents().inbox_rows, before);

    ui.click("documents-inbox-delete");
    ui.click("delete-document-confirm");

    let page = ui.documents();
    assert_eq!(page.dialog, None);
    assert_eq!(page.inbox_rows, before - 1);
    assert_ne!(page.selected_file.as_deref(), Some(STRONG_RECEIPT));
}
