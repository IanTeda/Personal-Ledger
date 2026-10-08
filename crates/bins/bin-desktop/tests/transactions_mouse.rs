//! The Transactions page driven by clicks: rows, chips and their clear buttons, the search box,
//! the add button and the filter popover's fields, options, status control and buttons. Only
//! elements tagged with `debug_selector` are clicked; state is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

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
