//! The Settings Tags page driven by keys: row selection, the add, edit and remove dialogs and
//! the merge dialog. State is read back from `Shell`.

mod common;

use bin_desktop::nav::InputMode;
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

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    let names = ui.settings().tag_names;
    assert_eq!(ui.settings().selected_tag.as_ref(), names.first());
    ui.press("j");
    assert_eq!(ui.settings().selected_tag.as_ref(), names.get(1));
    ui.press("shift-g");
    assert_eq!(ui.settings().selected_tag.as_ref(), names.last());
    ui.press("g g");
    assert_eq!(ui.settings().selected_tag.as_ref(), names.first());
}

#[gpui::test]
fn n_opens_add_tag_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddTag"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_and_selects_the_tag(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings().tag_names.len();

    ui.press("n");
    ui.press("z e b r a");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("zebra"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.tag_names.len(), before + 1);
    assert_eq!(page.selected_tag.as_deref(), Some("zebra"));
}

#[gpui::test]
fn enter_on_a_blank_name_keeps_the_dialog_open(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings().tag_names.len();

    ui.press("n enter");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("AddTag"));
    assert_eq!(page.tag_names.len(), before);
}

#[gpui::test]
fn e_edits_the_selected_tag_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let selected = ui.settings().selected_tag;

    ui.press("e");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditTag"));
    assert_eq!(page.dialog_name, selected);
    ui.press("x");
    assert_eq!(
        ui.settings().dialog_name,
        selected.map(|name| format!("{name}x"))
    );
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(
        page.selected_tag,
        page.tag_names.iter().find(|n| n.ends_with('x')).cloned()
    );
}

#[gpui::test]
fn x_removes_an_unused_tag_without_typing_its_name(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    // A new Tag is unused, so `enter` removes it without its name typed.
    ui.press("n z e b r a enter");
    let before = ui.settings().tag_names;
    assert_eq!(ui.settings().selected_tag.as_deref(), Some("zebra"));

    ui.press("x");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("RemoveTag"));
    assert_eq!(page.dialog_confirm.as_deref(), Some(""));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.tag_names.len(), before.len() - 1);
    assert!(!page.tag_names.iter().any(|name| name == "zebra"));
}

#[gpui::test]
fn a_used_tag_is_removed_only_once_its_name_is_typed(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings().tag_names;
    let used = "shared";
    let position = before.iter().position(|name| name == used);
    assert!(position.is_some());

    ui.press(&vec!["j"; position.unwrap_or(0)].join(" "));
    assert_eq!(ui.settings().selected_tag.as_deref(), Some(used));
    ui.press("x");
    ui.press("enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("RemoveTag"));
    ui.press("s h a r e d");
    assert_eq!(ui.settings().dialog_confirm.as_deref(), Some(used));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert!(!page.tag_names.iter().any(|name| name == used));
}

#[gpui::test]
fn m_on_a_flagged_duplicate_opens_merge_with_both_tags_chosen(app: &mut TestAppContext) {
    let mut ui = in_tags(app);

    // Work Trip duplicates work-trip; they sort last.
    ui.press("shift-g");
    assert_eq!(ui.settings().selected_tag.as_deref(), Some("work-trip"));
    ui.press("k");
    ui.press("m");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("MergeTags"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert!(page.merge_pair.is_some());
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn enter_in_the_merge_dialog_merges_the_duplicate_away(app: &mut TestAppContext) {
    let mut ui = in_tags(app);
    let before = ui.settings().tag_names.len();

    ui.press("shift-g k m");
    let Some((source, target)) = ui.settings().merge_pair else {
        panic!("a flagged duplicate opens Merge with both Tags chosen");
    };
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.tag_names.len(), before - 1);
    assert!(!page.tag_names.contains(&source));
    assert_eq!(page.selected_tag.as_ref(), Some(&target));
}
