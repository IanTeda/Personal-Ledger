//! The Settings View driven by clicks, across its pages: the Settings index, the resting view at
//! every Window Size, the docked index at Large and X-Large, and the Medium picker. Each page's own clicks are in its `settings_<page>_mouse.rs`. Only elements tagged
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
fn clicking_a_docked_index_entry_opens_its_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert!(!ui.settings_frame().page_focused, "the index shows");

    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings_frame().page, "Accounts");
    assert!(ui.settings_frame().page_focused);
    ui.click("settings-index-About");
    assert_eq!(ui.settings_frame().page, "About");
    ui.click("settings-index-General");
    assert_eq!(ui.settings_frame().page, "General");
}

#[gpui::test]
fn at_large_and_x_large_a_docked_click_on_a_page_keeps_the_keyboard_on_the_docked_index(
    app: &mut TestAppContext,
) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        ui.press("g s");
        ui.click("settings-index-Tags");
        assert!(ui.settings_frame().page_focused, "from the index, the page");

        ui.click("settings-index-Payees");

        let settings = ui.settings_frame();
        assert_eq!(settings.page, "Payees", "{window_size:?}");
        assert!(settings.dock_focused, "{window_size:?}: the keyboard stays");
        assert!(!settings.page_focused);
        assert!(ui.is_drawn("settings-page"), "the page shows");

        ui.click("settings-index-Payees");
        assert!(
            ui.settings_frame().dock_focused,
            "the page on show's row keeps the keyboard on the docked index"
        );
    }
}

#[gpui::test]
fn at_medium_an_index_row_opens_its_page_and_the_crumb_goes_back(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    ui.click("settings-index-page-Units");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "Units");
    assert!(settings.page_focused);
    assert!(ui.is_drawn("settings-page"));

    ui.click("settings-crumb-index");
    assert!(ui.is_drawn("settings-index-page"));
    assert_eq!(ui.settings_frame().page, "Units");
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
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "Tracing");
    assert_eq!(settings.picker_chosen, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn at_large_and_x_large_a_row_of_the_index_on_show_opens_its_page(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        ui.press("g s");
        assert!(ui.is_drawn("settings-index-page"), "{window_size:?}");
        assert!(ui.is_drawn("settings-index-General"), "docked beside it");

        ui.click("settings-index-page-Units");

        let settings = ui.settings_frame();
        assert_eq!(settings.page, "Units");
        assert!(settings.page_focused);
        assert!(ui.is_drawn("settings-units-table"));
    }
}

#[gpui::test]
fn the_rail_item_returns_an_open_page_to_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Tags");
    assert!(ui.settings_frame().page_focused);

    ui.click("primary-rail-Settings");

    let settings = ui.settings_frame();
    assert!(!settings.page_focused, "the rail item lands on the index");
    assert_eq!(settings.page, "Tags", "its cursor on the page left");
}

#[gpui::test]
fn at_x_large_the_docked_index_opens_the_capped_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.resize_to(WindowSize::XLarge);

    ui.click("settings-index-Display");
    assert_eq!(ui.settings_frame().page, "Display");
    assert!(ui.settings_frame().page_focused);
    let page = ui.bounds("settings-page");
    assert_eq!(
        page.size.width,
        gpui::px(840.0),
        "capped, the rest of the View empty"
    );
    assert!(!ui.is_drawn("settings-crumb-picker"));
}

/// Whether the wheel brought Data & backup, the last row, wholly inside the index page's scroll.
fn index_row_bottom_on_show(ui: &mut Harness<'_>) -> bool {
    let frame = ui.bounds("settings-index-page");
    ui.bounds("settings-index-page-DataBackup").bottom() <= frame.bottom()
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

    let before = ui.bounds("settings-index-page-DataBackup").top();
    ui.scroll_at(
        frame.center(),
        gpui::point(gpui::px(0.0), gpui::px(-2000.0)),
    );
    let after = ui.bounds("settings-index-page-DataBackup").top();
    assert!(after < before, "the wheel scrolls the rows up");
    assert!(index_row_bottom_on_show(&mut ui));

    ui.click("settings-index-page-DataBackup");
    assert_eq!(ui.settings_frame().page, "DataBackup");
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

#[gpui::test]
fn with_no_ledger_open_clicking_a_this_ledger_entry_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.press("g s");

    ui.click("settings-index-Ledger");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General", "the page on show stays");
    assert_eq!(settings.status_message.as_deref(), Some(common::NO_LEDGER));
    ui.click("settings-index-About");
    assert_eq!(ui.settings_frame().page, "About");
}

#[gpui::test]
fn with_no_ledger_open_clicking_a_this_ledger_row_on_the_medium_index_is_refused(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    ui.click("settings-index-page-Payees");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General");
    assert!(!settings.page_focused, "the index stays on show");
}
