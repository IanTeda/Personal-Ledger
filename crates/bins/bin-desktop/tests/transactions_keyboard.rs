//! The Transactions page driven by keys alone: rows and selection, the search box, the filter
//! popover and the not-yet-built row actions. Assertions read `TransactionsSnapshot`, `NavState`
//! and the status message back from `Shell`, never pixels.

mod common;

use bin_desktop::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

/// A Harness already on the Transactions page.
fn on_transactions(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g l");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|s| s.status_message().map(str::to_string))
}

#[gpui::test]
fn g_l_shows_every_row_with_the_default_chips(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    let page = ui.transactions();
    assert!(page.visible > 100, "the seed fills the table");
    assert_eq!(page.selected, 0);
    assert_eq!(page.search, "");
    assert_eq!(page.chips.len(), 6);
    assert!(page.chips.iter().all(|chip| !chip.active));
    assert!(!page.filters_changed);
    assert!(!page.filter_open);
}

#[gpui::test]
fn j_and_k_move_the_selection_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.press("k");
    assert_eq!(ui.transactions().selected, 0, "k stops on the first row");
    ui.press("j j j");
    assert_eq!(ui.transactions().selected, 3);
    ui.press("k");
    assert_eq!(ui.transactions().selected, 2);
}

#[gpui::test]
fn shift_g_jumps_to_the_last_row_and_g_g_back_to_the_first(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let last = ui.transactions().visible - 1;

    ui.press("shift-g");
    assert_eq!(ui.transactions().selected, last);
    ui.press("j");
    assert_eq!(ui.transactions().selected, last, "j stops on the last row");

    ui.press("g g");
    assert_eq!(ui.transactions().selected, 0);
}

#[gpui::test]
fn ctrl_d_and_ctrl_u_move_by_a_half_page(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.press("ctrl-d");
    let down = ui.transactions().selected;
    assert!(down > 0, "ctrl-d moves down");

    ui.press("ctrl-u");
    assert_eq!(ui.transactions().selected, 0);
}

#[gpui::test]
fn enter_n_and_e_say_they_are_not_yet_built(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.press("enter");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("open transaction \u{2014} not yet built")
    );
    ui.press("n");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("add transaction \u{2014} not yet built")
    );
    ui.press("e");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("edit transaction \u{2014} not yet built")
    );
}

#[gpui::test]
fn slash_searches_live_and_enter_keeps_the_text(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;
    ui.press("j j");

    ui.press("/");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Search);
    ui.press("k u r a");

    let found = ui.transactions();
    assert_eq!(found.search, "kura");
    assert!(found.visible > 0 && found.visible < all);
    assert_eq!(found.selected, 0, "typing puts the table back on row one");

    ui.press("enter");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
    assert_eq!(ui.transactions().search, "kura", "enter keeps the text");
    assert_eq!(ui.transactions().visible, found.visible);
}

#[gpui::test]
fn backspace_widens_the_search_and_escape_clears_it(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;
    ui.press("/ k u r a");
    let narrow = ui.transactions().visible;

    ui.press("backspace backspace");
    let wider = ui.transactions();
    assert_eq!(wider.search, "ku");
    assert!(wider.visible >= narrow);

    ui.press("escape");
    let cleared = ui.transactions();
    assert_eq!(cleared.search, "");
    assert_eq!(cleared.visible, all);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn f_opens_the_popover_on_account_and_escape_discards_the_draft(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);

    ui.press("f");
    let open = ui.transactions();
    assert!(open.filter_open);
    assert_eq!(open.filter_focus.as_deref(), Some("Account"));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Filter);

    ui.press("tab tab k u r a");
    ui.press("escape");

    let closed = ui.transactions();
    assert!(!closed.filter_open);
    assert!(!closed.filters_changed, "a cancelled draft changes nothing");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn tab_and_shift_tab_walk_the_popover_fields(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f");

    ui.press("tab");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Category"));
    ui.press("tab");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Payee"));
    ui.press("shift-tab");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Category"));
}

#[gpui::test]
fn typing_a_payee_and_pressing_enter_applies_the_filter(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;
    ui.press("j j f tab tab k u r a");

    ui.press("enter");

    let page = ui.transactions();
    assert!(!page.filter_open);
    assert!(page.filters_changed);
    assert!(page.visible > 0 && page.visible < all);
    assert!(page.payees.iter().all(|payee| payee.contains("Kura")));
    assert!(page.chips[2].active, "the payee chip turns accent");
    assert_eq!(page.selected, 0, "applying puts the table back on row one");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn the_status_control_steps_with_right_and_left(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    // Account, Category, Payee, Tag, From, To, then Status.
    ui.press("f tab tab tab tab tab tab");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Status"));

    ui.press("right");
    assert_eq!(ui.transactions().draft_status.as_deref(), Some("Open"));
    ui.press("right left");
    assert_eq!(ui.transactions().draft_status.as_deref(), Some("Open"));

    ui.press("enter");
    let page = ui.transactions();
    assert!(page.chips[5].active);
    assert!(page.filters_changed);
}

#[gpui::test]
fn ctrl_r_resets_the_draft_but_not_the_applied_filters(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f tab tab k u r a enter");
    let applied = ui.transactions();
    assert!(applied.filters_changed);

    ui.press("f ctrl-r");
    assert_eq!(ui.transactions().filter_focus.as_deref(), Some("Account"));
    assert_eq!(ui.transactions().visible, applied.visible);

    // Enter on a select opens its list instead, so step to the Payee text field to apply.
    ui.press("tab tab enter");
    assert!(
        !ui.transactions().filters_changed,
        "applying the reset draft returns every filter to its default"
    );
}

#[gpui::test]
fn an_unparseable_date_blocks_apply(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    // From is the fifth field; any text that is not a date.
    ui.press("f tab tab tab tab z z z");

    ui.press("enter");

    let page = ui.transactions();
    assert!(page.filter_open, "apply is a no-op while a date is invalid");
    assert!(!page.filters_changed);
}

#[gpui::test]
fn an_account_select_opens_with_enter_and_escape_closes_only_the_list(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    ui.press("f enter");
    assert!(ui.transactions().account_list_open);

    ui.press("escape");
    let page = ui.transactions();
    assert!(!page.account_list_open);
    assert!(page.filter_open, "the first Esc closes the list only");

    ui.press("escape");
    assert!(!ui.transactions().filter_open);
}

#[gpui::test]
fn choosing_an_account_with_the_keys_filters_to_it(app: &mut TestAppContext) {
    let mut ui = on_transactions(app);
    let all = ui.transactions().visible;

    // Open the list, step past "all accounts", commit the highlighted account, then step to the Payee text field, where enter applies.
    ui.press("f enter down enter");
    ui.press("tab tab enter");

    let page = ui.transactions();
    assert!(page.chips[0].active, "the account chip turns accent");
    assert!(page.visible < all);
}
