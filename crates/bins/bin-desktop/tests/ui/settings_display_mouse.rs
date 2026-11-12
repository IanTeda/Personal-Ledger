//! The Settings Display page driven by clicks: each segmented control, the status glyph radios,
//! the sidebar checkbox, the Toasts control and the Colour Theme cards, and the preview stacking
//! under the fields at the Medium floor. The date style, Row density, Status glyphs, Colour Theme and
//! Appearance are saved to the conf file (ADR-0053). Only elements tagged with `debug_selector` are
//! clicked.

use super::common;
use bin_desktop::shell::router::SettingsDisplaySnapshot;

use bin_desktop::WindowSize;
use common::Harness;
use gpui::TestAppContext;
use lib_colour_theme::ColourTheme;

fn on_display(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Display");
    assert_eq!(ui.settings_frame().page, "Display");
    ui
}

#[gpui::test]
fn clicking_a_date_format_option_selects_it(app: &mut TestAppContext) {
    let mut ui = on_display(app);

    ui.click("display-date-style-4");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>()
            .date_style
            .as_deref(),
        Some("Iso")
    );
    assert_eq!(ui.conf(), "[Display]\ndate_style = iso\n");
    ui.click("display-date-style-0");
    assert_eq!(ui.settings::<SettingsDisplaySnapshot>().date_style, None);
}

#[gpui::test]
fn clicking_a_status_glyph_radio_selects_it(app: &mut TestAppContext) {
    let mut ui = on_display(app);

    ui.click("display-status-glyphs-1");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "AsciiFallback"
    );
    assert_eq!(ui.conf(), "[Display]\nstatus_glyphs = ascii\n");
    ui.click("display-status-glyphs-0");
    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "Unicode"
    );
}

#[gpui::test]
fn clicking_a_status_glyph_radio_a_higher_file_or_variable_sets_changes_nothing(
    app: &mut TestAppContext,
) {
    let mut config = lib_config::Config::default();
    config.display.status_glyphs = lib_core::StatusGlyphs::AsciiFallback;
    let mut ui = Harness::with_configuration(
        app,
        &config,
        &[("PERSONAL_LEDGER_DISPLAY__STATUS_GLYPHS", "ascii")],
    );
    ui.press("g s");
    ui.click("settings-index-Display");
    assert!(ui.is_drawn("shadowed-status_glyphs"));

    ui.click("display-status-glyphs-0");

    assert_eq!(
        ui.settings::<SettingsDisplaySnapshot>().status_glyphs,
        "AsciiFallback"
    );
    assert_eq!(ui.conf(), "");
}

#[gpui::test]
fn clicking_the_sidebar_checkbox_toggles_it(app: &mut TestAppContext) {
    let mut ui = on_display(app);

    ui.click("display-start-sidebar-minimised");
    assert!(
        ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
    ui.click("display-start-sidebar-minimised");
    assert!(
        !ui.settings::<SettingsDisplaySnapshot>()
            .start_sidebar_minimised
    );
}

#[gpui::test]
fn clicking_an_appearance_option_changes_the_appearance(app: &mut TestAppContext) {
    let mut ui = on_display(app);

    ui.click("display-colour-appearance-1");
    assert_eq!(ui.colour_choice().1.as_deref(), Some("Dark"));
    assert_eq!(ui.conf(), "[Display]\ncolour_appearance = dark\n");
    ui.click("display-colour-appearance-0");
    assert_eq!(ui.colour_choice().1.as_deref(), Some("Light"));
}

#[gpui::test]
fn clicking_a_colour_theme_card_chooses_it(app: &mut TestAppContext) {
    let mut ui = on_display(app);

    let themes = ColourTheme::built_in();
    ui.click("display-colour-theme-3");
    assert_eq!(ui.colour_choice().0.as_deref(), Some(themes[3].id));
    assert_eq!(
        ui.conf(),
        format!("[Display]\ncolour_theme = {}\n", themes[3].id)
    );
    ui.click("display-colour-theme-1");
    assert_eq!(ui.colour_choice().0.as_deref(), Some(themes[1].id));
}

#[gpui::test]
fn clicking_a_shadowed_colour_theme_or_appearance_changes_nothing(app: &mut TestAppContext) {
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::Config::default(),
        &[
            ("PERSONAL_LEDGER_DISPLAY__COLOUR_THEME", "nord"),
            ("PERSONAL_LEDGER_DISPLAY__COLOUR_APPEARANCE", "dark"),
        ],
    );
    ui.press("g s");
    ui.click("settings-index-Display");
    assert!(ui.is_drawn("shadowed-colour_theme"));
    assert!(ui.is_drawn("shadowed-colour_appearance"));
    let before = ui.colour_choice();

    ui.click("display-colour-theme-3");
    ui.click("display-colour-appearance-1");

    assert_eq!(ui.colour_choice(), before);
    assert_eq!(ui.conf(), "");
}

#[gpui::test]
fn the_preview_stacks_under_the_fields_only_at_the_medium_floor(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");
    ui.click("settings-index-page-Display");
    assert_eq!(ui.settings_frame().page, "Display");

    let fields = ui.bounds("display-fields");
    let preview = ui.bounds("display-preview");
    assert!(preview.left() > fields.right(), "side by side at 960px");

    ui.resize(gpui::size(gpui::px(720.0), gpui::px(480.0)));
    let fields = ui.bounds("display-fields");
    let preview = ui.bounds("display-preview");
    assert!(preview.top() > fields.bottom(), "stacked at 720px");
    assert_eq!(preview.left(), fields.left());
}
