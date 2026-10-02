//! The Settings Display page driven by keys: walking the controls and into the Colour Theme
//! grid, changing each control with `h`/`l`, toggling the checkbox, and stepping back out.
//! State is read back from `Shell`.

mod common;

use common::Harness;
use gpui::TestAppContext;
use lib_colour_theme::ColourTheme;

/// Settings on the Display page (one press down the index) with focus stepped into it.
fn in_display(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j");
    assert_eq!(ui.settings().page, "Display");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn focus_starts_on_the_date_format_control(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    let page = ui.settings();
    assert_eq!(page.display_field, Some(0));
    assert_eq!(page.colour_theme_focus, None);
}

#[gpui::test]
fn j_and_k_walk_the_controls_and_stop_at_the_top(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j");
    assert_eq!(ui.settings().display_field, Some(1));
    ui.press("j j j");
    assert_eq!(ui.settings().display_field, Some(4));
    ui.press("k");
    assert_eq!(ui.settings().display_field, Some(3));
    ui.press("k k k k k");
    assert_eq!(ui.settings().display_field, Some(0));
}

#[gpui::test]
fn l_and_h_step_the_date_format_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    assert_eq!(ui.settings().date_style, None);
    ui.press("l");
    assert_eq!(ui.settings().date_style.as_deref(), Some("Short"));
    ui.press("l l l l");
    assert_eq!(ui.settings().date_style.as_deref(), Some("Iso"));
    ui.press("h h h h h h");
    assert_eq!(ui.settings().date_style, None);
}

#[gpui::test]
fn l_and_h_step_the_row_density(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j");
    assert_eq!(ui.settings().row_density, "Regular");
    ui.press("l l");
    assert_eq!(ui.settings().row_density, "Roomy");
    ui.press("h h h");
    assert_eq!(ui.settings().row_density, "Compact");
}

#[gpui::test]
fn l_and_h_step_the_status_glyphs(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j");
    assert_eq!(ui.settings().status_glyphs, "Unicode");
    ui.press("l");
    assert_eq!(ui.settings().status_glyphs, "AsciiFallback");
    ui.press("l");
    assert_eq!(ui.settings().status_glyphs, "AsciiFallback");
    ui.press("h");
    assert_eq!(ui.settings().status_glyphs, "Unicode");
}

#[gpui::test]
fn the_sidebar_checkbox_takes_l_h_enter_and_space(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j j");
    assert!(!ui.settings().start_sidebar_minimised);
    ui.press("l");
    assert!(ui.settings().start_sidebar_minimised);
    ui.press("h");
    assert!(!ui.settings().start_sidebar_minimised);
    ui.press("enter");
    assert!(ui.settings().start_sidebar_minimised);
    ui.press("space");
    assert!(!ui.settings().start_sidebar_minimised);
}

#[gpui::test]
fn h_turns_toasts_on_and_l_turns_them_off(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j j j");
    let was_on = ui.settings().toasts_on;
    ui.press("l");
    assert!(!ui.settings().toasts_on);
    ui.press("h");
    assert!(ui.settings().toasts_on);
    assert!(was_on);
}

#[gpui::test]
fn j_past_the_last_control_enters_the_grid_and_k_climbs_back(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j j j j");
    let page = ui.settings();
    assert_eq!(page.display_field, None);
    assert!(page.colour_theme_focus.is_some());
    ui.press("k");
    let page = ui.settings();
    assert_eq!(page.colour_theme_focus, None);
    assert_eq!(page.display_field, Some(4));
}

#[gpui::test]
fn the_grid_moves_with_l_and_h_and_enter_chooses_the_card(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j j");
    let start = ui.settings().colour_theme_focus.expect("grid has focus");

    ui.press("l");
    assert_eq!(ui.settings().colour_theme_focus, Some(start + 1));
    ui.press("enter");
    assert_eq!(
        ui.colour_choice().0.as_deref(),
        Some(ColourTheme::built_in()[start + 1].id)
    );
    ui.press("h");
    assert_eq!(ui.settings().colour_theme_focus, Some(start));
}

#[gpui::test]
fn escape_in_the_grid_climbs_to_the_last_control_then_back_to_the_index(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j j");

    ui.press("escape");
    assert_eq!(ui.settings().colour_theme_focus, None);
    assert_eq!(ui.settings().display_field, Some(4));
    ui.press("escape");
    let page = ui.settings();
    assert!(!page.page_focused);
    assert_eq!(page.display_field, None);
}

#[gpui::test]
fn h_in_the_first_grid_column_steps_out_to_the_index(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j j");
    // Home of the grid is the chosen card; walk left until focus leaves the grid.
    ui.press("h h h h h h");

    let page = ui.settings();
    assert!(!page.page_focused);
    assert_eq!(page.colour_theme_focus, None);
}
