//! The Settings Tags page driven by clicks: the add button, row selection, the row actions and
//! the dialogs' buttons. Only elements tagged with `debug_selector` are clicked.

use super::common;
use lib_core::TagId;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;

fn on_tags(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Tags");
    assert_eq!(ui.settings().page, "Tags");
    ui
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_tags(app);

    ui.click("settings-tags-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddTag"));
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Dialog);
    ui.click("add-tag-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn the_add_dialog_confirm_is_inert_until_a_name_is_typed(app: &mut TestAppContext) {
    let mut ui = on_tags(app);
    let before = ui.settings_tags().tag_names.len();

    ui.click("settings-tags-add");
    ui.click("add-tag-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddTag"));
    ui.press("z e b r a");
    ui.click("add-tag-confirm");
    let tags_page = ui.settings_tags();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(tags_page.tag_names.len(), before + 1);
    assert_eq!(tags_page.selected_tag.as_deref(), Some("zebra"));
}

#[gpui::test]
fn clicking_a_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_tags(app);

    let first = ui.settings_tags().selected_tag;
    ui.click(&format!("settings-tags-row-{}", TagId::seed(2)));
    let tags_page = ui.settings_tags();
    assert_eq!(tags_page.selected_tag.as_deref(), Some("shared"));
    assert_ne!(first, tags_page.selected_tag);
}

#[gpui::test]
fn the_edit_action_opens_its_dialog_with_the_name(app: &mut TestAppContext) {
    let mut ui = on_tags(app);

    ui.click(&format!("settings-tags-edit-{}", TagId::seed(2)));
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditTag"));
    assert_eq!(page.dialog_name.as_deref(), Some("shared"));
    ui.click("edit-tag-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn the_remove_action_opens_its_dialog_and_removes_an_unused_tag(app: &mut TestAppContext) {
    let mut ui = on_tags(app);
    // A new Tag is unused, so Remove needs no typed name.
    ui.press("l n z e b r a enter");
    let snapshot = ui.settings_tags();
    let before = snapshot.tag_names.len();
    let zebra = snapshot
        .tag_names
        .iter()
        .position(|name| name == "zebra")
        .map(|at| snapshot.tag_ids[at])
        .expect("the added Tag");

    ui.click(&format!("settings-tags-remove-{zebra}"));
    assert_eq!(ui.settings().dialog.as_deref(), Some("RemoveTag"));
    ui.click("remove-tag-cancel");
    assert_eq!(ui.settings().dialog, None);
    ui.click(&format!("settings-tags-remove-{zebra}"));
    ui.click("remove-tag-confirm");
    let tags_page = ui.settings_tags();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(tags_page.tag_names.len(), before - 1);
    assert!(!tags_page.tag_names.iter().any(|name| name == "zebra"));
}

#[gpui::test]
fn the_merge_action_on_a_flagged_duplicate_merges_it_away(app: &mut TestAppContext) {
    let mut ui = on_tags(app);
    let before = ui.settings_tags().tag_names.len();

    ui.click(&format!("settings-tags-merge-{}", TagId::seed(9)));
    let tags_page = ui.settings_tags();
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("MergeTags"));
    assert!(tags_page.merge_pair.is_some());
    ui.click("merge-tags-cancel");
    assert_eq!(ui.settings().dialog, None);
    ui.click(&format!("settings-tags-merge-{}", TagId::seed(9)));
    ui.click("merge-tags-confirm");
    let tags_page = ui.settings_tags();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(tags_page.tag_names.len(), before - 1);
}
