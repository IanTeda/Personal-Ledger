//! The Settings About page driven by clicks: the About index entry opens the page, and the
//! version line is drawn on it. About has no buttons or fields, so there is nothing else to
//! click. State is read back from `Root`.

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
fn clicking_the_about_index_entry_opens_the_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_ne!(ui.settings_frame().page, "About", "must not start on About");

    ui.click("settings-index-About");

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(
        page.page_focused,
        "a page with nothing to focus opens all the same"
    );
    assert_eq!(ui.dialog(), None);
    assert!(ui.is_drawn("settings-about-version"));
}

#[gpui::test]
fn clicking_about_from_another_page_opens_it(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Display");
    assert!(ui.settings_frame().page_focused);

    ui.click("settings-index-About");

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(
        page.dock_focused,
        "the keyboard stays on the docked index (#1110)"
    );
    assert!(ui.is_drawn("settings-about-version"));
}
