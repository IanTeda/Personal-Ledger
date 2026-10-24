//! The Settings View driven by clicks, across its pages: the index rail and its Medium index page
//! and picker. Each page's own clicks are in its `settings_<page>_mouse.rs`. Only elements tagged
//! with `debug_selector` are clicked; state is read back from `Root`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn clicking_an_index_entry_swaps_the_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");
    ui.click("settings-index-About");
    assert_eq!(ui.settings().page, "About");
    ui.click("settings-index-General");
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn at_medium_an_index_row_opens_its_page_and_the_crumb_goes_back(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    ui.click("settings-index-page-Units");
    let settings = ui.settings();
    assert_eq!(settings.page, "Units");
    assert!(settings.page_focused);
    assert!(ui.is_drawn("settings-page"));

    ui.click("settings-crumb-index");
    assert!(ui.is_drawn("settings-index-page"));
    assert_eq!(ui.settings().page, "Units");
}

#[gpui::test]
fn at_medium_the_caret_opens_the_picker_and_a_row_jumps_to_its_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter");

    ui.click("settings-crumb-picker");
    assert_eq!(mode(&mut ui), InputMode::Filter);
    assert!(ui.is_drawn("settings-picker-About"));

    ui.click("settings-picker-Tracing");
    let settings = ui.settings();
    assert_eq!(settings.page, "Tracing");
    assert_eq!(settings.picker_chosen, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn at_x_large_the_docked_index_swaps_the_capped_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.resize_to(WindowSize::XLarge);

    ui.click("settings-index-Display");
    assert_eq!(ui.settings().page, "Display");
    let page = ui.bounds("settings-page");
    assert_eq!(
        page.size.width,
        gpui::px(840.0),
        "capped, the rest of the View empty"
    );
    assert!(!ui.is_drawn("settings-crumb-picker"));
}

/// Whether the wheel brought About, the last row, wholly inside the index page's scroll.
fn index_row_bottom_on_show(ui: &mut Harness<'_>) -> bool {
    let frame = ui.bounds("settings-index-page");
    ui.bounds("settings-index-page-About").bottom() <= frame.bottom()
}

#[gpui::test]
fn at_medium_the_index_page_has_a_scrollbar_and_scrolls_by_wheel(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize(gpui::size(gpui::px(960.0), gpui::px(480.0)));
    ui.press("g s");

    let frame = ui.bounds("settings-index-page");
    let scrollbar = ui.bounds("settings-index-scrollbar");
    assert_eq!(
        scrollbar.right(),
        frame.right(),
        "the bar runs down the right edge"
    );
    assert_eq!(scrollbar.size.height, frame.size.height);

    let before = ui.bounds("settings-index-page-About").top();
    ui.scroll_at(
        frame.center(),
        gpui::point(gpui::px(0.0), gpui::px(-2000.0)),
    );
    let after = ui.bounds("settings-index-page-About").top();
    assert!(after < before, "the wheel scrolls the rows up");
    assert!(index_row_bottom_on_show(&mut ui));

    ui.click("settings-index-page-About");
    assert_eq!(ui.settings().page, "About");
}

#[gpui::test]
fn at_medium_every_index_row_carries_its_icon(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    for page in [
        "General",
        "Display",
        "Units",
        "Institutions",
        "Accounts",
        "Categories",
        "Tags",
        "Payees",
        "Documents",
        "Inventory",
        "SyncServer",
        "DataBackup",
        "Tracing",
        "About",
    ] {
        let row = ui.bounds(&format!("settings-index-page-{page}"));
        let icon = ui.bounds(&format!("settings-index-page-{page}-icon"));
        assert!(
            row.contains(&icon.center()) && icon.left() < row.center().x,
            "{page}'s icon leads its row"
        );
    }
}
