//! The Settings View's own clicks: an index entry hands focus back to the index, and a button
//! with nothing behind it says so on the status line. State is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

#[gpui::test]
fn clicking_an_index_entry_from_a_page_focuses_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings().page_focused);
    ui.click("settings-index-Display");
    let page = ui.settings();
    assert_eq!(page.page, "Display");
    assert!(!page.page_focused);
}

#[gpui::test]
fn clicking_add_price_source_says_it_is_not_built(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.click("settings-index-Units");
    ui.click("settings-add-price-source");
    assert!(ui.settings().status_message.is_some());
}
