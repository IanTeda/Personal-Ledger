//! The Settings View's own keys on the index and the General page: the index's ends, `/` with no
//! Search, and `Esc` on the index. State is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

#[gpui::test]
fn shift_g_and_g_g_jump_to_the_index_ends(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("shift-g");
    assert_eq!(ui.settings().page, "About");
    ui.press("g g");
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn slash_is_inert_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("/");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn escape_on_the_index_stays_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings().page_focused);
    ui.press("escape escape");
    let page = ui.settings();
    assert!(!page.page_focused);
    assert_eq!(page.page, "General");
    assert_eq!(ui.noun(), Noun::Settings);
}

#[gpui::test]
fn the_base_unit_select_starts_on_aud_and_lists_every_unit(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    let general = ui.settings_general();
    assert_eq!(general.base_unit.as_deref(), Some("aud"));
    assert_eq!(general.options, ["aud", "btc", "vas"]);
}

#[gpui::test]
fn j_and_k_in_the_page_set_the_base_unit_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    ui.press("j");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("btc"));
    ui.press("j j");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("vas"));
    ui.press("k");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("btc"));
    ui.press("k k");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("aud"));
}

#[gpui::test]
fn j_on_the_index_changes_the_page_and_not_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j");
    assert_eq!(ui.settings().page, "Display");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("aud"));
}

#[gpui::test]
fn a_base_unit_set_by_key_moves_the_units_pages_glyph(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    ui.press("j");
    ui.press("escape escape");
    ui.press("j");
    assert_eq!(ui.settings().page, "Display");
    ui.press("j");
    assert_eq!(ui.settings().page, "Units");
    assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
}

#[gpui::test]
fn a_deactivated_unit_is_not_offered_as_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert!(ui.settings_general().options.contains(&"btc".to_string()));

    ui.press("j j");
    ui.press("l");
    ui.press("j");
    ui.press("d");
    // Delete is refused for a Unit in use, which turns the Dialog into Deactivate.
    ui.press("b t c enter");
    ui.press("enter");
    ui.press("escape");
    ui.press("g g");

    let general = ui.settings_general();
    assert!(!general.options.contains(&"btc".to_string()));
    assert_eq!(general.base_unit.as_deref(), Some("aud"));
}
