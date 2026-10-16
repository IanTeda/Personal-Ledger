//! The Settings About page driven by clicks: the About index entry swaps the page in, and the
//! version line is drawn on it. About has no buttons or fields, so there is nothing else to
//! click. State is read back from `Shell`.

use super::common;

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
fn clicking_the_about_index_entry_swaps_to_the_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_ne!(ui.settings().page, "About", "must not start on About");

    ui.click("settings-index-About");

    let page = ui.settings();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    assert_eq!(page.dialog, None);
    assert!(ui.is_drawn("settings-about-version"));
}

#[gpui::test]
fn clicking_about_from_another_page_focuses_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Display");
    ui.press("l");
    assert!(ui.settings().page_focused);

    ui.click("settings-index-About");

    let page = ui.settings();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
}
