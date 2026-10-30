//! The Tags end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file. Add a
//! Tag and remove a Tag that Splits carry, restart, see both on the Tags page, and see a Tag the
//! Splits still carry counted as used, read from the Tags adapter. Run with the other smoke tests
//! at a parent ticket's close and before a merge to `main` (`mise run end-to-end-ui smoke`).

use super::common;
use bin_desktop::shell::router::SettingsTagsSnapshot;

use common::{Harness, config_for};
use gpui::TestAppContext;

/// Settings on the Tags page (six presses down the index) with focus stepped into it.
fn in_tags(ui: &mut Harness<'_>) {
    ui.press("g s");
    ui.press("j j j j j j");
    assert_eq!(ui.settings_frame().page, "Tags");
    ui.press("l");
}

/// Moves the Tags page's selection down to the row named `name`.
fn select(ui: &mut Harness<'_>, name: &str) {
    ui.press("g g");
    let position = ui
        .settings::<SettingsTagsSnapshot>()
        .tag_names
        .iter()
        .position(|row| row == name);
    assert!(position.is_some(), "the Tag is on the page");
    for _ in 0..position.unwrap_or(0) {
        ui.press("j");
    }
    assert_eq!(
        ui.settings::<SettingsTagsSnapshot>()
            .selected_tag
            .as_deref(),
        Some(name)
    );
}

#[gpui::test]
fn an_added_tag_and_a_removed_used_tag_are_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        in_tags(&mut ui);
        ui.press("n h o l i d a y enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");

        // `shared` is carried by Splits, so the file only lets it go once they are untagged.
        select(&mut ui, "shared");
        ui.press("x s h a r e d");
        assert_eq!(ui.dialog_value("confirm").as_deref(), Some("shared"));
        ui.press("enter");
        assert_eq!(ui.dialog(), None, "the Remove Dialog confirmed");
        ui.settle_writes();

        let names = ui.settings::<SettingsTagsSnapshot>().tag_names;
        assert!(names.iter().any(|name| name == "holiday"));
        assert!(!names.iter().any(|name| name == "shared"));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    in_tags(&mut ui);
    assert_eq!(ui.settings::<SettingsTagsSnapshot>().tag_names, names);

    // A Tag the stub's Splits still carry asks for its name: its usage is the adapter's count.
    select(&mut ui, "reimbursable");
    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveTag"));
    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("RemoveTag"),
        "a used Tag is not removed without its name"
    );
    ui.press("escape");
    select(&mut ui, "holiday");
    ui.press("x enter");
    assert_eq!(ui.dialog(), None, "a Tag nothing carries goes at once");
}
