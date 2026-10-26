//! The Settings Tags page driven by keys: row selection, the add, edit and remove dialogs and
//! the merge dialog. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Settings on the Tags page (seven presses down the index) with focus stepped into it.
fn in_tags(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j j j j j j");
    assert_eq!(ui.settings().page, "Tags");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

/// The open Merge dialog's chosen `(source, target)` names, once both are chosen.
fn merge_pair(ui: &mut Harness<'_>) -> Option<(String, String)> {
    Some((ui.dialog_value("source")?, ui.dialog_value("target")?))
}

#[gpui::test]
fn slash_is_inert_on_the_page(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    ui.press("/");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    let names = ui.settings_tags().tag_names;
    assert_eq!(ui.settings_tags().selected_tag.as_ref(), names.first());
    ui.press("j");
    assert_eq!(ui.settings_tags().selected_tag.as_ref(), names.get(1));
    ui.press("shift-g");
    assert_eq!(ui.settings_tags().selected_tag.as_ref(), names.last());
    ui.press("g g");
    assert_eq!(ui.settings_tags().selected_tag.as_ref(), names.first());
}

#[gpui::test]
fn n_opens_add_tag_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddTag"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_and_selects_the_tag(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings_tags().tag_names.len();

    ui.press("n");
    ui.press("z e b r a");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("zebra"));
    ui.press("enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), None);
    assert_eq!(tags_page.tag_names.len(), before + 1);
    assert_eq!(tags_page.selected_tag.as_deref(), Some("zebra"));
}

#[gpui::test]
fn enter_on_a_blank_name_keeps_the_dialog_open(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings_tags().tag_names.len();

    ui.press("n enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), Some("AddTag"));
    assert_eq!(tags_page.tag_names.len(), before);
}

#[gpui::test]
fn e_edits_the_selected_tag_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let selected = ui.settings_tags().selected_tag;

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditTag"));
    assert_eq!(ui.dialog_value("name"), selected);
    ui.press("x");
    assert_eq!(
        ui.dialog_value("name"),
        selected.map(|name| format!("{name}x"))
    );
    ui.press("enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        tags_page.selected_tag,
        tags_page
            .tag_names
            .iter()
            .find(|n| n.ends_with('x'))
            .cloned()
    );
}

#[gpui::test]
fn x_removes_an_unused_tag_without_typing_its_name(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    // A new Tag is unused, so `enter` removes it without its name typed.
    ui.press("n z e b r a enter");
    let before = ui.settings_tags().tag_names;
    assert_eq!(ui.settings_tags().selected_tag.as_deref(), Some("zebra"));

    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveTag"));
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some(""));
    ui.press("enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), None);
    assert_eq!(tags_page.tag_names.len(), before.len() - 1);
    assert!(!tags_page.tag_names.iter().any(|name| name == "zebra"));
}

#[gpui::test]
fn a_used_tag_is_removed_only_once_its_name_is_typed(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings_tags().tag_names;
    let used = "shared";
    let position = before.iter().position(|name| name == used);
    assert!(position.is_some());

    ui.press(&vec!["j"; position.unwrap_or(0)].join(" "));
    assert_eq!(ui.settings_tags().selected_tag.as_deref(), Some(used));
    ui.press("x");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("RemoveTag"));
    ui.press("s h a r e d");
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some(used));
    ui.press("enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), None);
    assert!(!tags_page.tag_names.iter().any(|name| name == used));
}

#[gpui::test]
fn m_on_a_flagged_duplicate_opens_merge_with_both_tags_chosen(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    // Work Trip duplicates work-trip; they sort last.
    ui.press("shift-g");
    assert_eq!(
        ui.settings_tags().selected_tag.as_deref(),
        Some("work-trip")
    );
    ui.press("k");
    ui.press("m");
    assert_eq!(ui.dialog(), Some("MergeTags"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert!(merge_pair(&mut ui).is_some());
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn enter_in_the_merge_dialog_merges_the_duplicate_away(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings_tags().tag_names.len();

    ui.press("shift-g k m");
    let Some((source, target)) = merge_pair(&mut ui) else {
        panic!("a flagged duplicate opens Merge with both Tags chosen");
    };
    ui.press("enter");
    let tags_page = ui.settings_tags();
    assert_eq!(ui.dialog(), None);
    assert_eq!(tags_page.tag_names.len(), before - 1);
    assert!(!tags_page.tag_names.contains(&source));
    assert_eq!(tags_page.selected_tag.as_ref(), Some(&target));
}

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn a_failed_write_puts_the_tags_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_tags(app);
    ui.press("g s");
    ui.press("j j j j j j");
    ui.press("l");
    let stored = ui.settings_tags().tag_names;

    ui.press("n z e b r a enter");
    assert_eq!(
        ui.settings_tags().tag_names.len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings_tags().tag_names, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to tags, so they are back as last saved"
    }));
}
