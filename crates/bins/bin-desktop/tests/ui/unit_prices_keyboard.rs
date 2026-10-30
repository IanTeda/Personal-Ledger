//! The Unit prices view on Settings › Units driven by keys, on in-memory Units ports. Wiring and
//! drawing only: `enter` on a Unit opens its prices view in place of the tables, `[` and `]` step
//! its window, `p` opens Set price and `Esc` returns to the tables on the same row. The window and
//! figure rules are the Presenter tests' in `lib-units`.

use super::common;
use bin_desktop::shell::router::{SettingsUnitsSnapshot, UnitPricesSnapshot};

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

/// Settings › Units with keyboard focus on the page and btc, the second Unit, selected.
fn on_btc(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j");
    assert_eq!(ui.settings_frame().page, "Units");
    ui.press("l");
    ui.press("j");
    assert_eq!(selected(&mut ui), "btc");
    ui
}

fn selected(ui: &mut Harness<'_>) -> String {
    ui.settings::<SettingsUnitsSnapshot>()
        .units
        .into_iter()
        .find(|unit| unit.selected)
        .map(|unit| unit.code)
        .unwrap_or_default()
}

#[gpui::test]
fn enter_opens_btcs_prices_drawn_in_place_of_the_tables(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("enter");

    let prices = ui
        .settings::<Option<UnitPricesSnapshot>>()
        .expect("enter opens the prices view");
    assert_eq!(prices.title, "btc · Bitcoin · prices");
    assert_eq!(prices.rows.len(), 52, "seeded btc has a year of weeks");
    for selector in [
        "unit-prices-header",
        "unit-prices-facts",
        "unit-prices-chart",
        "unit-prices-table",
        "unit-prices-row-0",
    ] {
        assert!(ui.is_drawn(selector), "{selector}");
    }
}

#[gpui::test]
fn brackets_step_the_window_a_month_and_stop_at_the_current_week(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("enter");
    let newest = |ui: &mut Harness<'_>| {
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .rows[0]
            .week
    };
    let current = newest(&mut ui);

    ui.press("[");
    let earlier = newest(&mut ui);
    assert!(earlier < current, "[ steps back");
    assert_eq!(
        ui.settings_frame().page,
        "Units",
        "[ steps the window, not the page"
    );
    assert!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .can_go_later
    );

    ui.press("]");
    assert_eq!(newest(&mut ui), current);
    ui.press("]");
    assert_eq!(newest(&mut ui), current, "never past the current week");
}

#[gpui::test]
fn j_walks_the_weeks_and_the_status_line_names_the_views_keys(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("enter");
    ui.press("j");
    assert!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .rows[1]
            .selected
    );

    let hints = ui.status().expect("the page has a status line").hints;
    for (key, action) in [("[", "earlier"), ("]", "later"), ("esc", "back")] {
        assert!(
            hints.contains(&(key.to_string(), action.to_string())),
            "{key} missing from {hints:?}"
        );
    }
}

#[gpui::test]
fn escape_returns_to_the_tables_with_the_same_unit_selected(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("enter");
    ui.press("escape");

    assert!(ui.settings::<Option<UnitPricesSnapshot>>().is_none());
    assert_eq!(ui.settings_frame().page, "Units");
    assert!(
        ui.settings_frame().page_focused,
        "Esc closed only the prices view"
    );
    assert_eq!(selected(&mut ui), "btc");
    assert!(ui.is_drawn("settings-units-table"));
}

#[gpui::test]
fn a_unit_with_no_prices_shows_the_empty_state_and_p_sets_one(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("j");
    assert_eq!(selected(&mut ui), "vas");
    ui.press("enter");
    assert_eq!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .empty
            .as_deref(),
        Some("vas has no weekly prices yet.")
    );
    assert!(ui.is_drawn("unit-prices-empty"));

    ui.press("p");
    assert_eq!(ui.dialog(), Some("SetPrice"));
    ui.press("4 2");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    let prices = ui
        .settings::<Option<UnitPricesSnapshot>>()
        .expect("the prices view stays open");
    assert_eq!(prices.empty, None);
    assert_eq!(prices.rows[0].close, "42.00");
}
