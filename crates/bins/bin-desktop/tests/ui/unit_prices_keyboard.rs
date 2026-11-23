//! The Unit prices view on Settings › Units driven by keys, on in-memory Units ports. Wiring and
//! drawing only: `enter` on a Unit opens its prices view in place of the tables, `[` and `]` step
//! its window, `z` steps its grain, fetching finer prices in place through a fake price feed, `p`
//! opens Set price and `Esc` returns to the tables on the same row. The window, grain and figure
//! rules are the Presenter tests' in `lib-units`.

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
    ui.press("j j j j j");
    assert_eq!(ui.settings_frame().page, "Units");
    ui.press("l");
    ui.press("j");
    assert_eq!(selected(&mut ui), "BTC");
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
    assert_eq!(prices.title, "BTC · Bitcoin · prices");
    assert_eq!(prices.rows.len(), 52, "seeded BTC has a year of weeks");
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
    assert_eq!(selected(&mut ui), "BTC");
    assert!(ui.is_drawn("settings-units-table"));
}

#[gpui::test]
fn a_unit_with_no_prices_shows_the_empty_state_and_p_sets_one(app: &mut TestAppContext) {
    let mut ui = on_btc(app);
    ui.press("j");
    assert_eq!(selected(&mut ui), "VAS");
    ui.press("enter");
    assert_eq!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .empty
            .as_deref(),
        Some("VAS has no weekly prices yet.")
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

/// BTC priced as the Ledger opens, so the check on open sends no round its finer requests would
/// wait behind.
#[expect(clippy::expect_used, reason = "the price is a literal decimal")]
fn btc_priced_now(fakes: &mut lib_database::FakeStores) {
    let opened = lib_core::Clock::fixed(common::today()).now();
    for unit in &mut fakes.units.units.units {
        if unit.id == lib_core::UnitId::SEED_BTC {
            unit.record_price("1".parse().expect("a decimal"), common::today(), opened);
        }
    }
}

/// CoinGecko's daily `[ms, price]` pairs for every day from 120 days before today to tomorrow,
/// whichever of them the window holds.
fn coingecko_days() -> String {
    let today = common::today();
    let pairs: Vec<String> = (-120..=1)
        .map(|offset| {
            let day = today + chrono::Duration::days(offset);
            let ms = day
                .and_time(chrono::NaiveTime::MIN)
                .and_utc()
                .timestamp_millis();
            format!("[{ms},{}]", 1000 + offset)
        })
        .collect();
    format!(r#"{{"prices":[{}]}}"#, pairs.join(","))
}

#[gpui::test]
fn z_steps_btc_to_daily_prices_fetched_in_place_then_hourly_and_back_to_weekly(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), btc_priced_now);
    ui.press("g s");
    ui.press("j j j j j");
    ui.press("l");
    ui.press("j");
    assert_eq!(selected(&mut ui), "BTC");
    ui.press("enter");
    let prices = |ui: &mut Harness<'_>| {
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("the prices view is open")
    };
    assert_eq!(prices(&mut ui).grain, lib_units::Grain::Week);
    assert!(ui.is_drawn("unit-prices-grain-day"));

    ui.press("z");
    let day = prices(&mut ui);
    assert_eq!(day.grain, lib_units::Grain::Day);
    assert_eq!(day.chart_heading, "Daily close");
    assert_eq!(
        day.empty.as_deref(),
        Some("Fetching daily prices from CoinGecko…")
    );
    assert!(ui.is_drawn("unit-prices-empty"));
    assert!(
        transport
            .urls()
            .last()
            .is_some_and(|url| url.contains("/market_chart/range")),
        "{:?}",
        transport.urls()
    );

    transport.answer_last(200, &coingecko_days());
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    let day = prices(&mut ui);
    assert_eq!(day.empty, None);
    assert!(
        day.rows.len() > 80,
        "about 3 months of days: {}",
        day.rows.len()
    );
    assert!(ui.is_drawn("unit-prices-chart"));
    assert!(ui.is_drawn("unit-prices-row-0"));

    ui.press("[");
    assert!(prices(&mut ui).can_go_later);

    ui.press("z");
    assert_eq!(prices(&mut ui).grain, lib_units::Grain::Hour);
    ui.press("z");
    let week = prices(&mut ui);
    assert_eq!(week.grain, lib_units::Grain::Week);
    assert_eq!(week.rows.len(), 52);
}

#[gpui::test]
fn a_failed_daily_request_says_why_in_place_with_no_toast(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), btc_priced_now);
    ui.press("g s");
    ui.press("j j j j j");
    ui.press("l");
    ui.press("j");
    ui.press("enter");
    ui.press("z");
    transport.answer_last(404, "{}");
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    let prices = ui.settings::<Option<UnitPricesSnapshot>>().expect("open");
    assert_eq!(
        prices.empty.as_deref(),
        Some("Couldn't fetch: CoinGecko has no bitcoin in AUD. Press z to go back to weekly.")
    );
    assert!(ui.is_drawn("unit-prices-empty"));
    let toasts = ui.toasts();
    assert!(toasts.visible.is_empty() && toasts.history.is_empty());
}
