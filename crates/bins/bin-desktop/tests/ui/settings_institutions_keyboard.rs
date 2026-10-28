//! The Settings Institutions page driven by keys: row selection, `n` for the Add dialog and its
//! typing, `enter` to save and `escape` to cancel, `d` for the Delete dialog in its two states, and
//! the legend `?` lists. State is read back
//! from `Root`.

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, page: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click(&format!("settings-index-{page}"));
    assert_eq!(ui.settings().page, page);
    ui
}

/// The Institutions page with focus stepped into it.
fn in_institutions(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = on_page(app, "Institutions");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

/// The status line's hint keys, in order; empty when the line has no page content.
fn hint_keys(ui: &mut Harness<'_>) -> Vec<String> {
    ui.status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default()
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message())
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    let names: Vec<String> = ui
        .settings_institutions()
        .institutions
        .into_iter()
        .map(|row| row.0)
        .collect();
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.first()
    );
    ui.press("j");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.get(1)
    );
    ui.press("k");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.first()
    );
    ui.press("shift-g");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.last()
    );
    ui.press("g g");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.first()
    );
}

#[gpui::test]
fn the_page_lists_seven_institutions_the_personal_one_first(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    let page = ui.settings_institutions();
    assert_eq!(page.institutions.len(), 7);
    assert_eq!(
        page.institutions.first(),
        Some(&("Personal".to_string(), "cash".to_string()))
    );
    assert_eq!(page.selected_institution.as_deref(), Some("Personal"));
}

#[gpui::test]
fn n_opens_the_add_dialog_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn adding_an_institution_selects_it_and_raises_a_toast(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    ui.press("n");
    ui.press("b a n k");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("bank")
    );
    assert_eq!(
        ui.toasts().history.first().map(|toast| toast.text.clone()),
        Some("Added bank".to_string())
    );
}

/// Backspaces the Edit dialog's name empty and types `name` (single characters, `space` between
/// words) in its place.
fn retype_name(ui: &mut Harness<'_>, name: &str) {
    let current = ui.dialog_value("name").unwrap_or_default();
    for _ in current.chars() {
        ui.press("backspace");
    }
    let keys: Vec<String> = name
        .chars()
        .map(|ch| {
            if ch == ' ' {
                "space".to_string()
            } else {
                ch.to_string()
            }
        })
        .collect();
    ui.press(&keys.join(" "));
}

#[gpui::test]
fn e_opens_the_edit_dialog_filled_with_the_selected_institution(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    ui.press("e");

    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(
        ui.dialog_value("title").as_deref(),
        Some("Edit institution")
    );
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Personal"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Cash"));
}

#[gpui::test]
fn enter_saves_an_edit_selects_the_row_and_raises_a_toast(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);
    ui.press("j");

    ui.press("e");
    retype_name(&mut ui, "Amex");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    let page = ui.settings_institutions();
    assert_eq!(page.selected_institution.as_deref(), Some("Amex"));
    assert_eq!(page.institutions[1].0, "Amex");
    assert_eq!(page.institutions.len(), 7);
    assert_eq!(
        ui.toasts().history.first().map(|toast| toast.text.clone()),
        Some("Saved Amex".to_string())
    );
}

#[gpui::test]
fn a_rename_to_a_name_another_holds_is_refused_and_a_change_of_capitals_is_accepted(
    app: &mut TestAppContext,
) {
    let mut ui = in_institutions(app);
    let before = ui.settings_institutions().institutions;

    ui.press("j e");
    retype_name(&mut ui, "american express");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(ui.settings_institutions().institutions, before);

    retype_name(&mut ui, "anz banking group");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().institutions[1].0,
        "anz banking group"
    );
}

#[gpui::test]
fn escape_cancels_an_edit_with_nothing_changed_and_focus_back_on_the_row(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);
    let before = ui.settings_institutions().institutions;

    ui.press("e");
    retype_name(&mut ui, "Changed");
    ui.press("escape");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_institutions().institutions, before);
    assert!(ui.settings().page_focused);
    ui.press("j");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("ANZ Banking Group")
    );
}

/// Types `name` into the Delete Dialog's confirmation: single characters, `space` between words.
fn type_name(ui: &mut Harness<'_>, name: &str) {
    let keys: Vec<String> = name
        .chars()
        .map(|ch| {
            if ch == ' ' {
                "space".to_string()
            } else {
                ch.to_string()
            }
        })
        .collect();
    ui.press(&keys.join(" "));
}

#[gpui::test]
fn d_opens_the_refused_dialog_for_an_institution_an_account_names(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);
    let before = ui.settings_institutions().institutions;
    ui.press("j");

    ui.press("d");

    assert_eq!(ui.dialog(), Some("DeleteInstitution"));
    assert_eq!(
        ui.dialog_value("title").as_deref(),
        Some("Delete institution \u{2014} ANZ Banking Group")
    );
    assert_eq!(ui.dialog_value("refused").as_deref(), Some("true"));
    assert_eq!(
        ui.dialog_value("notice").as_deref(),
        Some("Used by 2 accounts")
    );
    assert_eq!(
        ui.dialog_value("accounts").as_deref(),
        Some("ANZ Everyday, ANZ Offset")
    );
    assert_eq!(ui.dialog_value("more").as_deref(), Some(""));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));

    type_name(&mut ui, "ANZ Banking Group");
    assert_eq!(
        ui.dialog_value("confirm").as_deref(),
        Some(""),
        "nothing to type back"
    );
    ui.press("enter");
    assert_eq!(ui.dialog(), None, "Enter is Close");
    assert_eq!(ui.settings_institutions().institutions, before);
}

#[gpui::test]
fn escape_cancels_a_delete_with_nothing_changed_and_focus_back_on_the_row(
    app: &mut TestAppContext,
) {
    let mut ui = in_institutions(app);
    let before = ui.settings_institutions().institutions;
    ui.press("shift-g");

    ui.press("d");
    type_name(&mut ui, "Superannuation Fund");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    ui.press("escape");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_institutions().institutions, before);
    assert!(ui.settings().page_focused);
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("Superannuation Fund")
    );
}

#[gpui::test]
fn delete_waits_for_the_name_typed_back_exactly_then_removes_the_row(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);
    ui.press("shift-g");

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteInstitution"));
    assert_eq!(ui.dialog_value("refused").as_deref(), Some("false"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));

    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("DeleteInstitution"),
        "nothing typed back yet"
    );
    type_name(&mut ui, "superannuation fund");
    assert_eq!(
        ui.dialog_value("can_confirm").as_deref(),
        Some("false"),
        "capitals count"
    );
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteInstitution"));

    for _ in 0.."superannuation fund".len() {
        ui.press("backspace");
    }
    type_name(&mut ui, "Superannuation Fund");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    let page = ui.settings_institutions();
    assert_eq!(page.institutions.len(), 6);
    assert!(
        page.institutions
            .iter()
            .all(|row| row.0 != "Superannuation Fund")
    );
    assert_eq!(
        page.selected_institution.as_deref(),
        Some("Cryptocurrency Exchange"),
        "the last row's neighbour is the one before"
    );
    assert_eq!(
        ui.toasts().history.first().map(|toast| toast.text.clone()),
        Some("Deleted Superannuation Fund".to_string())
    );
}

#[gpui::test]
fn deleting_a_row_that_is_not_last_selects_the_next_row(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);
    for name in ["alpha", "beta"] {
        ui.press("n");
        type_name(&mut ui, name);
        ui.press("enter");
    }
    ui.press("k");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("alpha")
    );

    ui.press("d");
    type_name(&mut ui, "alpha");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("beta")
    );
}

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = in_institutions(app);

    assert_eq!(hint_keys(&mut ui), ["j/k", "e", "d", "n"]);
    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn a_rebound_new_key_shows_on_the_status_line_and_in_the_cheat_sheet(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("new", "ctrl+n"));
    ui.press("g s");
    ui.click("settings-index-Institutions");
    ui.press("l");
    assert_eq!(hint_keys(&mut ui), ["j/k", "e", "d", "ctrl+n"]);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn add_institution_types_a_name_and_enter_adds_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("b a n k");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("bank"));
    ui.press("backspace");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("ban"));
    ui.press("enter");
    let institutions_page = ui.settings_institutions();
    assert_eq!(ui.dialog(), None);
    assert_eq!(institutions_page.institutions.len(), before + 1);
    assert_eq!(
        institutions_page
            .institutions
            .last()
            .map(|row| row.0.as_str()),
        Some("ban")
    );
}

#[gpui::test]
fn add_institution_enter_does_nothing_without_a_name(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.settings_institutions().institutions.len(), before);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn a_failed_write_puts_the_institutions_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_institutions(app);
    ui.press("g s");
    ui.click("settings-index-Institutions");
    let stored = ui.settings_institutions().institutions;

    ui.click("settings-add-institution");
    ui.press("b a n k enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().institutions.len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings_institutions().institutions, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text
                == "Couldn't save the change to institutions, so they are back as last saved"
    }));
}

#[gpui::test]
fn a_failed_delete_puts_the_institution_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_institutions(app);
    ui.press("g s");
    ui.click("settings-index-Institutions");
    ui.press("l");
    let stored = ui.settings_institutions().institutions;
    ui.press("shift-g");

    ui.press("d");
    type_name(&mut ui, "Superannuation Fund");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().institutions.len(),
        stored.len() - 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings_institutions().institutions, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text
                == "Couldn't save the change to institutions, so they are back as last saved"
    }));
}

#[gpui::test]
fn tab_moves_to_the_usual_unit_where_the_arrows_pick_and_tab_returns_to_the_name(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.press("b a n k");
    assert_eq!(ui.dialog_value("focused").as_deref(), Some("Name"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("focused").as_deref(), Some("UsualUnit"));
    ui.press("down");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("aud"));
    ui.press("down");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("btc"));
    ui.press("up");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("aud"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("focused").as_deref(), Some("Name"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("bank"));
}

#[gpui::test]
fn the_usual_unit_list_opens_on_enter_commits_on_enter_and_the_form_saves_it(
    app: &mut TestAppContext,
) {
    let mut ui = in_institutions(app);

    ui.press("n");
    ui.press("b a n k");
    ui.press("tab");
    ui.press("enter");
    ui.press("down down");
    ui.press("enter");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("btc"));
    assert_eq!(
        ui.dialog(),
        Some("AddInstitution"),
        "enter picked, it did not confirm"
    );
    ui.press("tab");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);

    ui.press("e");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("bank"));
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("btc"));
}

#[gpui::test]
fn typing_on_the_usual_unit_select_does_not_reach_the_name_or_the_shell(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.press("b a n k");
    ui.press("tab");
    ui.press("x y z");

    assert_eq!(ui.dialog_value("name").as_deref(), Some("bank"));
    assert_eq!(ui.dialog(), Some("AddInstitution"));
}
