//! The Transactions page driven by clicks: rows, chips and their clear buttons, the search box,
//! the add button, the filter popover's fields, options, status control and buttons, and the
//! Import step's selects and focus. Only elements tagged with `debug_selector` are clicked; state
//! is read back from `Shell`.

#![expect(
    clippy::expect_used,
    reason = "test support: an Import step that doesn't open, or has nothing to review, should fail the test loudly"
)]

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{FocusZone, InputMode, Noun},
};
use common::Harness;
use gpui::{TestAppContext, point, px, size};

fn on_transactions(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g l");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui
}

#[gpui::test]
fn clicking_a_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.click("transactions-row-2");
    assert_eq!(ui.transactions().selected, 2);
    ui.click("transactions-row-0");
    assert_eq!(ui.transactions().selected, 0);
}

#[gpui::test]
fn the_add_button_says_it_is_not_yet_built(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.click("transactions-add");

    assert_eq!(
        ui.read(|s| s.status_message().map(str::to_string))
            .as_deref(),
        Some("add transaction \u{2014} not yet built")
    );
}

#[gpui::test]
fn clicking_the_search_box_enters_search_mode(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.click("transactions-search");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Search);

    ui.press("k u r a");
    assert_eq!(ui.transactions().search, "kura");
}

#[gpui::test]
fn clicking_a_chip_opens_the_popover_on_its_field(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.click("transactions-chip-2");

    let page = ui.transactions();
    assert!(page.filter_open);
    assert_eq!(page.filter_focus.as_deref(), Some("Payee"));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Filter);
}

#[gpui::test]
fn clicking_a_text_field_focuses_it_and_apply_filters(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;
    ui.press("f");

    ui.click("filter-tag");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Tag"));
    ui.click("filter-payee");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Payee"));
    ui.press("k u r a");

    ui.click("filter-apply");

    let page = ui.transactions();
    assert!(!page.filter_open);
    assert!(page.visible > 0 && page.visible < all);
    assert!(page.chips[2].active);
}

#[gpui::test]
fn the_status_segments_set_the_draft(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f");

    // The segments run All, Open, Cleared, Reconciled.
    ui.click("filter-status-2");
    let page = ui.transactions();
    assert_eq!(page.draft_status.as_deref(), Some("Cleared"));
    assert_eq!(page.filter_focus.as_deref(), Some("Status"));

    ui.click("filter-apply");
    assert!(ui.transactions().chips[5].active);
}

#[gpui::test]
fn clicking_an_account_opens_its_list_and_an_option_filters(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;
    ui.press("f");

    ui.click("filter-account");
    assert!(ui.transactions().account_list_open);

    ui.click("filter-account-option-1");
    assert!(!ui.transactions().account_list_open);

    ui.click("filter-apply");
    let page = ui.transactions();
    assert!(page.chips[0].active);
    assert!(page.visible < all);
}

#[gpui::test]
fn reset_clears_the_draft_and_leaves_the_applied_filters(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f tab tab k u r a enter");
    let applied = ui.transactions();
    assert!(applied.filters_changed);

    ui.press("f");
    ui.click("filter-reset");
    assert_eq!(ui.transactions().visible, applied.visible);
    assert!(
        ui.transactions().filter_open,
        "reset leaves the popover open"
    );

    ui.click("filter-apply");
    assert!(!ui.transactions().filters_changed);
}

#[gpui::test]
fn a_chip_s_clear_button_resets_only_that_filter(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f tab tab k u r a enter");
    assert!(ui.transactions().chips[2].active);

    ui.click("transactions-chip-clear-2");

    let page = ui.transactions();
    assert!(!page.chips[2].active);
    assert!(!page.filters_changed);
    assert!(
        !page.filter_open,
        "the clear button does not open the popover"
    );
}

#[gpui::test]
fn clear_filters_resets_every_filter_but_keeps_the_search(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("/ k enter");
    ui.press("f tab tab k u r a enter");
    assert!(ui.transactions().filters_changed);

    ui.click("transactions-clear-filters");

    let page = ui.transactions();
    assert!(!page.filters_changed);
    assert!(page.chips.iter().all(|chip| !chip.active));
    assert_eq!(page.search, "k", "clear filters leaves the search alone");
}

/// The `:import` step, with the first row whose Payee and Category selects are drawn.
fn on_import(app: &mut TestAppContext) -> (Harness<'_>, usize) {
    let mut ui = Harness::new(app);
    ui.press(": i m p o r t enter");
    let import = ui.import().expect("`:import` opens the step");
    let row = import
        .rows
        .iter()
        .position(|row| row.status == "NeedsReview")
        .expect("the seeded statement has a row to review");
    (ui, row)
}

#[gpui::test]
fn clicking_an_import_select_opens_it_and_an_option_closes_it(app: &mut TestAppContext) {
    let (mut ui, row) = on_import(app);

    ui.click(&format!("import-payee-{row}"));
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Payee")
    );
    ui.click(&format!("import-payee-{row}-option-0"));
    assert_eq!(ui.import().expect("import is open").open_select, None);
}

#[gpui::test]
fn clicking_an_open_import_select_again_closes_it(app: &mut TestAppContext) {
    let (mut ui, row) = on_import(app);

    ui.click(&format!("import-category-{row}"));
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Category")
    );
    ui.click(&format!("import-category-{row}"));
    assert_eq!(ui.import().expect("import is open").open_select, None);
}

#[gpui::test]
fn continue_does_nothing_while_a_row_needs_review(app: &mut TestAppContext) {
    let (mut ui, _) = on_import(app);

    ui.click("import-continue");
    assert!(ui.import().is_some());
    assert!(ui.toasts().history.is_empty());
}

#[gpui::test]
fn clicking_in_the_page_gives_the_view_focus(app: &mut TestAppContext) {
    let (mut ui, _) = on_import(app);
    ui.press("shift-tab");
    assert_ne!(ui.read(|s| s.nav().focus()), FocusZone::View);

    ui.click("import-row-3");
    assert_eq!(ui.read(|s| s.nav().focus()), FocusZone::View);
    assert_eq!(ui.import().expect("import is open").selected, 3);
}

#[gpui::test]
fn at_large_the_table_fits_its_space(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.resize_to(WindowSize::Large);

    let space = ui.bounds("view-x-scroll");
    let content = ui.bounds("view-content");
    assert_eq!(content.size.width, space.size.width);
}

#[gpui::test]
fn at_the_medium_floor_the_table_scrolls_sideways_inside_the_view(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.resize(size(px(720.0), px(1040.0)));

    let space = ui.bounds("view-x-scroll");
    let content = ui.bounds("view-content");
    assert!(
        content.size.width > space.size.width,
        "the table ({:?}) is wider than its space ({:?})",
        content.size.width,
        space.size.width
    );
    let toggle = ui.bounds("topbar-rail-toggle");
    let status = ui.bounds("status-right");
    let search = ui.bounds("transactions-search");

    ui.scroll_at(search.center(), point(px(-120.0), px(0.0)));

    assert!(
        ui.bounds("view-content").origin.x < content.origin.x,
        "the View scrolled sideways"
    );
    assert!(ui.bounds("transactions-search").origin.x < search.origin.x);
    assert_eq!(
        ui.bounds("topbar-rail-toggle"),
        toggle,
        "the top bar never scrolls"
    );
    assert_eq!(
        ui.bounds("status-right"),
        status,
        "the status line never scrolls"
    );
}
