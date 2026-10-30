//! The Settings Payees page driven by keys: row selection, and the add, edit and delete dialogs
//! with what confirming each one does. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsPayeesSnapshot;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Settings on the Payees page (seven presses down the index) with focus stepped into it.
fn in_payees(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    to_payees(&mut ui);
    ui
}

fn to_payees(ui: &mut Harness<'_>) {
    ui.press("g s");
    ui.press("j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Payees");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
}

/// The newest Toast's text, if one is showing.
fn last_toast(ui: &mut Harness<'_>) -> Option<String> {
    ui.toasts().history.first().map(|toast| toast.text.clone())
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    let names = ui.settings::<SettingsPayeesSnapshot>().payee_names;
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_ref(),
        names.first()
    );
    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_ref(),
        names.get(1)
    );
    ui.press("k");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_ref(),
        names.first()
    );
    ui.press("shift-g");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_ref(),
        names.last()
    );
    ui.press("g g");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_ref(),
        names.first()
    );
}

#[gpui::test]
fn n_opens_add_payee_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddPayee"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_and_selects_the_payee(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    let before = ui.settings::<SettingsPayeesSnapshot>().payee_names.len();

    ui.press("n");
    ui.press("z o o");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("zoo"));
    ui.press("enter");
    let payees_page = ui.settings::<SettingsPayeesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(payees_page.payee_names.len(), before + 1);
    assert_eq!(payees_page.selected_payee.as_deref(), Some("zoo"));
}

#[gpui::test]
fn enter_on_a_blank_name_keeps_the_dialog_open(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    let before = ui.settings::<SettingsPayeesSnapshot>().payee_names.len();

    ui.press("n enter");
    assert_eq!(ui.dialog(), Some("AddPayee"));
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>().payee_names.len(),
        before
    );
}

#[gpui::test]
fn e_edits_the_selected_payee_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    let selected = ui.settings::<SettingsPayeesSnapshot>().selected_payee;

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditPayee"));
    assert_eq!(ui.dialog_value("name"), selected);
    ui.press("x");
    let renamed = selected.map(|name| format!("{name}x"));
    assert_eq!(ui.dialog_value("name"), renamed);
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>().selected_payee,
        renamed
    );
}

#[gpui::test]
fn a_failed_write_puts_the_payees_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_payees(app);
    ui.press("g s");
    ui.press("j j j j j j j");
    ui.press("l");
    let stored = ui.settings::<SettingsPayeesSnapshot>().payee_names;

    ui.press("n z o o enter");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>().payee_names.len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings::<SettingsPayeesSnapshot>().payee_names, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to payees, so they are back as last saved"
    }));
}

/// Settings on the Payees page with the row named `name` selected.
fn on_payee(ui: &mut Harness<'_>, name: &str) {
    to_payees(ui);
    let row = ui
        .settings::<SettingsPayeesSnapshot>()
        .payee_names
        .iter()
        .position(|payee| payee == name)
        // A name the stub doesn't seed selects the first row, which the assert below refuses.
        .unwrap_or_default();
    ui.press("g g");
    for _ in 0..row {
        ui.press("j");
    }
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some(name)
    );
}

#[gpui::test]
fn a_delete_the_files_foreign_keys_refuse_puts_the_payee_back_with_an_error_toast(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = common::config_for(&dir.path().join("ledger.pldb"));
    let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
    let writer = storage.writer.clone();
    let mut ui = Harness::with_storage_counting_payees_unused(app, storage);
    on_payee(&mut ui, "Woolworths");
    let stored = ui.settings::<SettingsPayeesSnapshot>().payee_names;

    ui.press("d");
    assert_eq!(
        ui.dialog_value("action").as_deref(),
        Some("Delete"),
        "the miscount offers Delete for a Payee the stub's Splits name"
    );
    ui.press("W o o l w o r t h s enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        last_toast(&mut ui).as_deref(),
        Some("Deleted payee Woolworths")
    );
    writer.flush().expect("the writer is running");
    ui.settle_writes();

    assert_eq!(ui.settings::<SettingsPayeesSnapshot>().payee_names, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to payees, so they are back as last saved"
    }));
}

#[gpui::test]
fn d_on_a_payee_whose_use_cant_be_counted_offers_only_deactivate(app: &mut TestAppContext) {
    let mut ui = Harness::with_unreadable_payee_usage(app);
    // Nothing names Netflix, so only the failed count keeps it from Delete.
    on_payee(&mut ui, "Netflix");

    ui.press("d");

    assert_eq!(ui.dialog_value("action").as_deref(), Some("Deactivate"));
    assert_eq!(
        ui.dialog_value("warning").as_deref(),
        Some(
            "What uses this payee couldn't be checked, so it can't be deleted — only deactivated."
        )
    );
}

#[gpui::test]
fn d_on_a_payee_an_import_created_and_named_on_a_split_offers_only_deactivate(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    // The Import commit books a Split naming its new Payee after the harness counted usage.
    super::import_keyboard::commit_the_candle_shop_as_a_new_payee(&mut ui);
    on_payee(&mut ui, "Aussie Candle Co");

    ui.press("d");

    assert_eq!(ui.dialog(), Some("DeletePayee"));
    assert_eq!(ui.dialog_value("action").as_deref(), Some("Deactivate"));
}

#[gpui::test]
fn d_deletes_an_unused_payee_once_its_name_is_typed(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    // A new Payee is unused, so the dialog hard-deletes it.
    ui.press("n z o o enter");
    let before = ui.settings::<SettingsPayeesSnapshot>().payee_names.len();

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeletePayee"));
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeletePayee"));
    ui.press("z o o");
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some("zoo"));
    ui.press("enter");
    let payees_page = ui.settings::<SettingsPayeesSnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(payees_page.payee_names.len(), before - 1);
    assert!(!payees_page.payee_names.iter().any(|name| name == "zoo"));
    assert!(payees_page.selected_payee.is_some());
    assert_eq!(last_toast(&mut ui).as_deref(), Some("Deleted payee zoo"));
}

#[gpui::test]
fn d_on_a_used_payee_deactivates_it_and_keeps_it_listed(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    let before = ui.settings::<SettingsPayeesSnapshot>().payee_names;
    let used = "Woolworths";
    let position = before.iter().position(|name| name == used);
    assert!(position.is_some());

    ui.press(&vec!["j"; position.unwrap_or(0)].join(" "));
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some(used)
    );
    ui.press("d");
    ui.press("W o o l w o r t h s");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings::<SettingsPayeesSnapshot>().payee_names, before);
    assert_eq!(
        last_toast(&mut ui).as_deref(),
        Some("Deactivated payee Woolworths")
    );
}

#[gpui::test]
fn enter_shows_the_selected_payees_transactions_in_one_commit(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    ui.press("j");
    let selected = ui
        .settings::<SettingsPayeesSnapshot>()
        .selected_payee
        .expect("a stub Payee is selected");

    let before = ui.commits();
    ui.press("enter");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.commits(), before + 1, "the key is one input");
    let page = ui.transactions();
    assert_eq!(page.selected, 0);
    assert!(page.filters_changed, "the list is filtered to the Payee");
    assert!(page.visible > 0, "the stub books to {selected}");
    // A split Transaction shows its first Payee and a `+n`, so the rows aren't all the one name.
    assert!(
        page.payees.contains(&selected),
        "{selected}'s Transactions show: {:?}",
        page.payees
    );

    // Back on Payees, the same row is still selected.
    ui.press("g s");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>().selected_payee,
        Some(selected)
    );
}

#[gpui::test]
fn h_steps_back_to_the_index_and_the_page_keys_stop(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    ui.press("h");
    assert!(!ui.settings_frame().page_focused);
    ui.press("n");
    assert_eq!(ui.dialog(), None);
    ui.press("j");
    assert_eq!(
        ui.settings_frame().page,
        "Documents",
        "j walks the index again"
    );
}

#[gpui::test]
fn slash_is_inert_on_the_page(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    ui.press("/");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert!(ui.settings_frame().page_focused);
}

#[gpui::test]
fn a_selection_survives_leaving_the_page_and_coming_back(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    ui.press("j j");
    let selected = ui.settings::<SettingsPayeesSnapshot>().selected_payee;

    ui.press("h");
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>().selected_payee,
        selected
    );
}

/// The status line's hint keys, in order; empty when the line has no page content.
fn hint_keys(ui: &mut Harness<'_>) -> Vec<String> {
    ui.status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default()
}

/// The status line's right-hand text; empty when the line has no page content.
fn status_right(ui: &mut Harness<'_>) -> String {
    ui.status().map(|status| status.right).unwrap_or_default()
}

#[gpui::test]
fn the_status_line_shows_the_page_legend_and_the_payee_count(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    assert_eq!(hint_keys(&mut ui), ["j/k", "enter", "e", "d", "n"]);
    // A leading count, then the without-default-category tally after a middle dot.
    let right = status_right(&mut ui);
    assert!(right.starts_with(|c: char| c.is_ascii_digit()), "{right}");
    assert!(right.contains(" \u{b7} "), "{right}");
}

#[gpui::test]
fn the_dialogs_swap_their_hints_into_the_status_line(app: &mut TestAppContext) {
    let mut ui = in_payees(app);
    let page_right = status_right(&mut ui);

    ui.press("n");
    assert_eq!(hint_keys(&mut ui), ["esc", "enter", "tab"]);
    assert_eq!(status_right(&mut ui), page_right);
    ui.press("escape");
    assert_eq!(hint_keys(&mut ui), ["j/k", "enter", "e", "d", "n"]);

    ui.press("e");
    assert_eq!(hint_keys(&mut ui), ["esc", "enter", "tab"]);
    ui.press("escape");

    ui.press("d");
    assert_eq!(hint_keys(&mut ui), ["esc", "enter"]);
    ui.press("escape");
    assert_eq!(hint_keys(&mut ui), ["j/k", "enter", "e", "d", "n"]);
}

#[gpui::test]
fn tab_reaches_the_default_category_and_down_picks_a_leaf_ctrl_enter_saves(
    app: &mut TestAppContext,
) {
    let mut ui = in_payees(app);

    ui.press("n z o o tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("DefaultCategory"));
    ui.press("x");
    assert_eq!(
        ui.dialog_value("name").as_deref(),
        Some("zoo"),
        "the select swallows typing"
    );
    ui.press("down");
    assert_eq!(ui.dialog_value("category").as_deref(), Some("Rent"));
    ui.press("ctrl-enter");
    assert_eq!(ui.dialog(), None, "submit confirms from the select");
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some("zoo")
    );

    ui.press("e");
    assert_eq!(ui.dialog_value("category").as_deref(), Some("Rent"));
}

#[gpui::test]
fn enter_in_the_rule_input_adds_a_chip_and_backspace_takes_it_back(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    ui.press("n z o o shift-tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Rule"));
    ui.press("z o o x enter");
    assert_eq!(ui.dialog(), Some("AddPayee"), "enter added a chip");
    assert_eq!(ui.dialog_value("rules").as_deref(), Some("ZOOX"));
    ui.press("backspace");
    assert_eq!(ui.dialog_value("rules").as_deref(), Some(""));

    ui.press("w o o l i e s enter");
    assert_eq!(ui.dialog_value("rules").as_deref(), Some(""));
    assert_eq!(
        ui.dialog_value("error").as_deref(),
        Some("WOOLIES is already a match rule on Woolworths")
    );
}

#[gpui::test]
fn an_override_of_next_field_moves_the_field_and_frees_tab(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("next_field", "ctrl+n"));
    ui.press("g s");
    ui.press("j j j j j j j");
    ui.press("l");
    ui.press("n");

    ui.press("ctrl-n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("DefaultCategory"));
    ui.press("tab");
    assert_eq!(
        ui.dialog_value("field").as_deref(),
        Some("DefaultCategory"),
        "tab is no longer bound"
    );
}

#[gpui::test]
fn a_documents_payee_link_lands_on_the_selected_payee(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g f");
    for _ in 0..200 {
        if ui.documents().selected_file.as_deref() == Some("aami-car-policy.pdf") {
            break;
        }
        ui.press("j");
    }
    ui.press("L");
    assert_eq!(ui.documents().dialog, Some("picker-follow"));

    ui.press("enter");

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "Payees");
    assert!(ui.settings_frame().page_focused);
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some("AAMI")
    );
    ui.press("e");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("AAMI"));
}

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = in_payees(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn the_cheat_sheet_shows_a_rebound_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("new", "ctrl+n"));
    ui.press("g s");
    ui.press("j j j j j j j");
    ui.press("l");
    assert_eq!(hint_keys(&mut ui), ["j/k", "enter", "e", "d", "ctrl+n"]);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}
