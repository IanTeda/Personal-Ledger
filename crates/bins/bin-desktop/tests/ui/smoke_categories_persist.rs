//! The Categories end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! Add a Category, restart, see it on the Categories page, and offered in a Payee's Default
//! category select, read from the Categories adapter. Run with the other smoke tests at a parent
//! ticket's close and before a merge to `main` (`mise run end-to-end-ui smoke`).

use super::common;

use common::{Harness, config_for};
use gpui::TestAppContext;

/// Settings on the page `presses` down the index, named `page`, with focus stepped into it.
fn on_page(ui: &mut Harness<'_>, presses: &str, page: &str) {
    ui.press("g s");
    ui.press(presses);
    assert_eq!(ui.settings().page, page);
    ui.press("l");
}

/// Whether the Add payee Dialog's Default category select reaches `name`, stepping down its
/// leaves from the first. Settings opens on its first page.
fn default_category_offers(ui: &mut Harness<'_>, name: &str) -> bool {
    on_page(ui, "j j j j j j j", "Payees");
    ui.press("n z o o tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("DefaultCategory"));
    let mut offered = false;
    for _ in 0..30 {
        ui.press("down");
        if ui.dialog_value("category").as_deref() == Some(name) {
            offered = true;
            break;
        }
    }
    ui.press("escape");
    offered
}

#[gpui::test]
fn an_added_category_is_there_after_a_restart_and_a_payee_may_default_to_it(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        on_page(&mut ui, "j j j j j", "Categories");
        ui.press("n p e t s enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        let names = ui.settings_categories().category_names;
        assert!(names.iter().any(|name| name == "pets"));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert!(
        default_category_offers(&mut ui, "pets"),
        "the Payees read the new leaf from the Categories adapter"
    );
    // The index keeps its place, two below Categories.
    on_page(&mut ui, "k k", "Categories");
    assert_eq!(ui.settings_categories().category_names, names);
}
