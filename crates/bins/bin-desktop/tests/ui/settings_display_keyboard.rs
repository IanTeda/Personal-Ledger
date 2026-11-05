//! The Settings Display page driven by keys: walking the controls and into the Colour Theme
//! grid, changing each control with `h`/`l`, toggling the checkbox, and stepping back out. The date
//! style, Row density, Status glyphs and Colour Theme are saved to the conf file (ADR-0053), unless
//! a source above it sets them, and the page starts on what the file holds. State is read back from
//! `Root` and the conf file.

use super::common;
use bin_desktop::shell::router::SettingsDisplaySnapshot;

use common::Harness;
use gpui::TestAppContext;
use lib_colour_theme::ColourTheme;

/// Settings on the Display page (one press down the index) with focus stepped into it.
fn in_display(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Display");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui
}

#[gpui::test]
fn focus_starts_on_the_date_format_control(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.display_field, Some(0));
    assert_eq!(display_page.colour_theme_focus, None);
}

#[gpui::test]
fn j_and_k_walk_the_controls_and_stop_at_the_top(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(1)
    );
    ui.press("j j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(3)
    );
    ui.press("k");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(2)
    );
    ui.press("k k k k k");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(0)
    );
}

#[gpui::test]
fn l_and_h_step_the_date_format_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>()
            .date_style
            .as_deref(),
        Some("Short")
    );
    ui.press("l l l l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>()
            .date_style
            .as_deref(),
        Some("Iso")
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\ndate_style = iso\n");
    ui.press("h h h h h h");
    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
    assert_eq!(ui.conf(), "[Personal-Ledger]\ndate_style = default\n");
}

#[gpui::test]
fn a_date_style_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_PERSONAL_LEDGER__DATE_STYLE";
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::PersonalLedgerConfig::default(),
        &[(var, "iso")],
    );
    ui.press("g s j l");
    assert!(ui.is_drawn("shadowed-date_style"));

    ui.press("l");

    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
    assert_eq!(ui.conf(), "");
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.contains(var)));
}

#[gpui::test]
fn display_starts_on_the_date_style_and_colours_the_conf_file_holds(app: &mut TestAppContext) {
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.date_style = Some(lib_core::DateStyle::Long);
    config.colour_theme = Some("nord".into());
    config.colour_appearance = lib_colour_theme::ColourAppearance::Light;
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s j");

    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>()
            .date_style
            .as_deref(),
        Some("Long")
    );
    assert_eq!(
        ui.colour_choice(),
        (Some("nord".to_string()), Some("Light".to_string()))
    );
}

#[gpui::test]
fn l_and_h_step_the_row_density(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().row_density,
        "Regular"
    );
    ui.press("l l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().row_density,
        "Roomy"
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\nrow_density = roomy\n");
    ui.press("h h h");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().row_density,
        "Compact"
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\nrow_density = compact\n");
}

#[gpui::test]
fn a_row_density_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_PERSONAL_LEDGER__ROW_DENSITY";
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::PersonalLedgerConfig::default(),
        &[(var, "roomy")],
    );
    ui.press("g s j l");
    assert!(ui.is_drawn("shadowed-row_density"));
    assert!(!ui.is_drawn("shadowed-status_glyphs"));

    ui.press("j l");

    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().row_density,
        "Regular"
    );
    assert_eq!(ui.conf(), "");
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.contains(var)));
}

#[gpui::test]
fn display_starts_on_the_row_density_and_status_glyphs_the_conf_file_holds(
    app: &mut TestAppContext,
) {
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.row_density = lib_core::RowDensity::Compact;
    config.status_glyphs = lib_core::StatusGlyphs::AsciiFallback;
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s j");

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.row_density, "Compact");
    assert_eq!(display_page.status_glyphs, "AsciiFallback");
    assert_eq!(ui.conf(), "", "a restore writes nothing");
}

#[gpui::test]
fn l_and_h_step_the_status_glyphs(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "Unicode"
    );
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "AsciiFallback"
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\nstatus_glyphs = ascii\n");
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "AsciiFallback"
    );
    ui.press("h");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "Unicode"
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\nstatus_glyphs = unicode\n");
}

#[gpui::test]
fn the_sidebar_checkbox_takes_l_h_enter_and_space(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j j");
    assert!(
        !ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    ui.press("l");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    ui.press("h");
    assert!(
        !ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    ui.press("enter");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    ui.press("space");
    assert!(
        !ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
}

#[gpui::test]
fn j_past_the_last_control_enters_the_grid_and_k_climbs_back(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j j j j");
    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.display_field, None);
    assert!(display_page.colour_theme_focus.is_some());
    ui.press("k");
    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.colour_theme_focus, None);
    assert_eq!(display_page.display_field, Some(3));
}

#[gpui::test]
fn the_grid_moves_with_l_and_h_and_enter_chooses_the_card(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    let start = ui
        .settings::<SettingsDisplaySnapshot>()
        .colour_theme_focus
        .expect("grid has focus");

    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        Some(start + 1)
    );
    ui.press("enter");
    let chosen = ColourTheme::built_in()[start + 1].id;
    assert_eq!(ui.colour_choice().0.as_deref(), Some(chosen));
    assert_eq!(
        ui.conf(),
        format!("[Personal-Ledger]\ncolour_theme = {chosen}\n")
    );
    ui.press("h");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        Some(start)
    );
}

#[gpui::test]
fn escape_in_the_grid_climbs_to_the_last_control_then_back_to_the_index(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j");

    ui.press("escape");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        None
    );
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(3)
    );
    ui.press("escape");
    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    let page = ui.settings_frame();
    assert!(!page.page_focused);
    assert_eq!(display_page.display_field, None);
}

#[gpui::test]
fn h_in_the_first_grid_column_steps_out_to_the_index(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    // Home of the grid is the chosen card; walk left until focus leaves the grid.
    ui.press("h h h h h h");

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    let page = ui.settings_frame();
    assert!(!page.page_focused);
    assert_eq!(display_page.colour_theme_focus, None);
}

#[gpui::test]
fn settings_display_from_the_palette_puts_focus_back_on_the_first_control(
    app: &mut TestAppContext,
) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .colour_theme_focus
            .is_some()
    );

    ui.press(": s e t t i n g s space d i s p l a y enter");

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.colour_theme_focus, None);
    assert_eq!(display_page.display_field, Some(0));
    assert!(ui.settings_frame().page_focused);
}

#[gpui::test]
fn the_theme_and_appearance_commands_save_to_the_conf_file(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(": t h e m e space n o r d enter");
    ui.press(": a p p e a r a n c e space l i g h t enter");

    assert_eq!(
        ui.colour_choice(),
        (Some("nord".to_string()), Some("Light".to_string()))
    );
    assert_eq!(
        ui.conf(),
        "[Personal-Ledger]\ncolour_theme = nord\ncolour_appearance = light\n"
    );
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.starts_with("colour_appearance saved to")));
}
