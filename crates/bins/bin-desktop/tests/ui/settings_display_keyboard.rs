//! The Settings Display page driven by keys: walking the controls and into the Colour Theme
//! grid, changing each control with `h`/`l`, toggling the checkbox, and stepping back out. The date
//! style, Row density, Status glyphs and Colour Theme are saved to the conf file (ADR-0053), unless
//! a source above it sets them, and the page starts on what the file holds. State is read back from
//! `Root` and the conf file. `u` undoes each change and `ctrl-r` redoes it, putting the file's raw
//! line back and repainting the window in the earlier colours (#1246).

use super::common;
use bin_desktop::shell::router::SettingsDisplaySnapshot;
use lib_settings::DisplayField;

use bin_desktop::navigation::nav::FocusZone;
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
    assert_eq!(display_page.display_field, Some(DisplayField::DateStyle));
    assert_eq!(display_page.colour_theme_focus, None);
}

#[gpui::test]
fn j_and_k_walk_the_controls_and_stop_at_the_top(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(DisplayField::RowDensity)
    );
    ui.press("j j");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(DisplayField::StartSidebar)
    );
    ui.press("k");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(DisplayField::StatusGlyphs)
    );
    ui.press("k k k k k");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().display_field,
        Some(DisplayField::DateStyle)
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
    assert_eq!(ui.conf(), "[Display]\ndate_style = iso\n");
    ui.press("h h h h h h");
    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
    assert_eq!(ui.conf(), "[Display]\ndate_style = default\n");
}

#[gpui::test]
fn a_date_style_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_DISPLAY__DATE_STYLE";
    let mut ui = Harness::with_configuration(app, &lib_config::Config::default(), &[(var, "iso")]);
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
    let mut config = lib_config::Config::default();
    config.display.date_style = Some(lib_core::DateStyle::Long);
    config.display.colour_theme = Some("nord".into());
    config.display.colour_appearance = lib_colour_theme::ColourAppearance::Light;
    config.display.row_density = lib_core::RowDensity::Compact;
    config.display.status_glyphs = lib_core::StatusGlyphs::AsciiFallback;
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s j");

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.date_style.as_deref(), Some("Long"));
    assert_eq!(display_page.row_density, "Compact");
    assert_eq!(display_page.status_glyphs, "AsciiFallback");
    assert_eq!(ui.conf(), "", "a restore writes nothing");
    assert_eq!(
        ui.colour_choice(),
        (Some("nord".to_string()), Some("Light".to_string()))
    );
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
fn the_sidebar_checkbox_is_written_to_the_conf_file_and_named_on_the_status_line(
    app: &mut TestAppContext,
) {
    let mut ui = in_display(app);

    ui.press("j j j l");

    assert_eq!(ui.conf(), "[Display]\nstart_sidebar_minimised = true\n");
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.starts_with("start_sidebar_minimised saved to")));
}

#[gpui::test]
fn a_variable_overrides_the_sidebar_checkbox_and_nothing_is_written(app: &mut TestAppContext) {
    let mut config = lib_config::Config::default();
    config.display.start_sidebar_minimised = true;
    let mut ui = Harness::with_configuration(
        app,
        &config,
        &[("PERSONAL_LEDGER_DISPLAY__START_SIDEBAR_MINIMISED", "true")],
    );
    ui.press("g s");
    ui.press("j");
    ui.press("l");
    ui.press("j j j");

    ui.press("h");

    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised,
        "the variable still sets it"
    );
    assert_eq!(ui.conf(), "");
}

#[gpui::test]
fn u_undoes_the_sidebar_checkbox_and_ctrl_r_redoes_it_without_moving_the_rail(
    app: &mut TestAppContext,
) {
    let mut ui = in_display(app);
    let before = ui.rail();
    ui.press("j j j l");
    assert_eq!(ui.rail(), before, "the rail applies at the next start");
    assert_eq!(ui.conf(), "[Display]\nstart_sidebar_minimised = true\n");

    ui.press("u");
    assert_eq!(ui.conf(), "[Display]\n");
    assert!(
        !ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    assert_eq!(ui.rail(), before);

    ui.press("ctrl-r");
    assert_eq!(ui.conf(), "[Display]\nstart_sidebar_minimised = true\n");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    assert_eq!(ui.rail(), before);
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
    assert_eq!(display_page.display_field, Some(DisplayField::StartSidebar));
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
    assert_eq!(ui.conf(), format!("[Display]\ncolour_theme = {chosen}\n"));
    ui.press("h");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        Some(start)
    );
}

/// Where the Colour Theme grid's focus is, `None` outside the grid.
fn grid_focus(ui: &mut Harness<'_>) -> Option<usize> {
    ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus
}

/// The grid's columns follow the width the page is drawn at, as a Window Size change leaves it. The
/// grid measures itself as it paints and the page redraws with the count straight after, so the
/// first key after a resize steps by the columns on screen, with nothing framing the page per key
/// (#1222).
#[gpui::test]
fn j_in_the_grid_steps_a_row_of_the_columns_the_window_size_leaves(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    assert_eq!(
        grid_focus(&mut ui),
        Some(0),
        "the grid opens on the chosen card"
    );
    ui.press("j");
    assert_eq!(
        grid_focus(&mut ui),
        Some(0),
        "every card fits one row of the X-Large body, so there is no row below"
    );

    // Medium's 720px floor, whose body fits fewer cards a row than there are themes.
    ui.resize(gpui::size(gpui::px(720.0), gpui::px(480.0)));
    assert_eq!(
        grid_focus(&mut ui),
        Some(0),
        "a resize leaves the grid's focus"
    );
    ui.press("j");
    let row = grid_focus(&mut ui).unwrap_or_default();
    assert!(
        (1..ColourTheme::built_in().len()).contains(&row),
        "j steps one row of the narrower grid, {row} cards on"
    );
    ui.press("k");
    assert_eq!(
        grid_focus(&mut ui),
        Some(0),
        "k climbs back to the card it left"
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
        Some(DisplayField::StartSidebar)
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

/// `key` in the Colour Theme grid leaves it, then moves the focus off the View as it does
/// anywhere.
fn leaves_the_grid_and_moves_the_focus(app: &mut TestAppContext, key: &str) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .colour_theme_focus
            .is_some()
    );

    ui.press(key);

    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        None
    );
    assert_ne!(ui.read(|shell| shell.nav().focus()), FocusZone::View);
}

#[gpui::test]
fn tab_leaves_the_grid_and_moves_the_focus_on(app: &mut TestAppContext) {
    leaves_the_grid_and_moves_the_focus(app, "tab");
}

#[gpui::test]
fn shift_tab_leaves_the_grid_and_moves_the_focus_back(app: &mut TestAppContext) {
    leaves_the_grid_and_moves_the_focus(app, "shift-tab");
}

/// `key` in the Colour Theme grid leaves it, the page keeping the keyboard as Settings scrolls.
fn leaves_the_grid_and_scrolls(app: &mut TestAppContext, key: &str) {
    let mut ui = in_display(app);
    ui.press("j j j j");

    ui.press(key);

    let display_page = ui.settings::<SettingsDisplaySnapshot>();
    assert_eq!(display_page.colour_theme_focus, None);
    assert!(ui.settings_frame().page_focused);
}

#[gpui::test]
fn ctrl_d_leaves_the_grid(app: &mut TestAppContext) {
    leaves_the_grid_and_scrolls(app, "ctrl-d");
}

#[gpui::test]
fn ctrl_u_leaves_the_grid(app: &mut TestAppContext) {
    leaves_the_grid_and_scrolls(app, "ctrl-u");
}

#[gpui::test]
fn a_key_the_grid_does_not_bind_leaves_it_where_it_is(app: &mut TestAppContext) {
    let mut ui = in_display(app);
    ui.press("j j j j");
    let start = ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus;

    ui.press("space");
    ui.press("shift-g");

    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().colour_theme_focus,
        start
    );
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
    assert_eq!(display_page.display_field, Some(DisplayField::DateStyle));
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
        "[Display]\ncolour_theme = nord\ncolour_appearance = light\n"
    );
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.starts_with("colour_appearance saved to")));
}

/// The status line's message.
fn status_message(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(bin_desktop::shell::router::Root::status_message)
}

#[gpui::test]
fn u_undoes_a_display_field_and_ctrl_r_redoes_it(app: &mut TestAppContext) {
    let mut ui = in_display(app);

    ui.press("l");
    assert_eq!(ui.conf(), "[Display]\ndate_style = short\n");

    ui.press("u");
    assert_eq!(ui.conf(), "[Display]\n");
    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
    assert!(
        status_message(&mut ui)
            .is_some_and(|text| text.starts_with("Undid Date format changed to "))
    );

    ui.press("ctrl-r");
    assert_eq!(ui.conf(), "[Display]\ndate_style = short\n");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>()
            .date_style
            .as_deref(),
        Some("Short")
    );
}

#[gpui::test]
fn undoing_a_theme_and_an_appearance_repaints_the_window_in_the_earlier_ones(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.press(": t h e m e space n o r d enter");
    ui.press(": a p p e a r a n c e space l i g h t enter");

    ui.press("u");
    assert_eq!(ui.colour_choice().1, Some("System".to_string()));
    assert_eq!(ui.colour_choice().0.as_deref(), Some("nord"));
    assert_eq!(ui.conf(), "[Display]\ncolour_theme = nord\n");

    ui.press("u");
    assert_eq!(ui.colour_choice().0, None);
    assert_eq!(ui.conf(), "[Display]\n");

    ui.press("ctrl-r ctrl-r");
    assert_eq!(
        ui.colour_choice(),
        (Some("nord".to_string()), Some("Light".to_string()))
    );
    assert_eq!(
        ui.conf(),
        "[Display]\ncolour_theme = nord\ncolour_appearance = light\n"
    );
}

#[gpui::test]
fn a_grid_pick_is_undone_and_repeating_the_theme_makes_no_entry(app: &mut TestAppContext) {
    let mut ui = Harness::with_conf_file(app, "[Display]\ncolour_theme = nord\n");
    ui.press(": t h e m e space n o r d enter");

    ui.press("u");

    assert_eq!(status_message(&mut ui).as_deref(), Some("Nothing to undo"));
    assert_eq!(ui.conf(), "[Display]\ncolour_theme = nord\n");
}
