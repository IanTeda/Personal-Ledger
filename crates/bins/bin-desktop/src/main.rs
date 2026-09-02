//! Personal Ledger desktop app -- feasibility-cycle demo.
//!
//! Demonstrates chart rendering in `GPUI` (ADR-0007) against dummy data, per
//! FC-DESKTOP-002, using `gpui-component`'s chart widgets. `gpui-component` ships
//! `LineChart`, `PieChart` (with a real donut via `inner_radius`), `CandlestickChart`,
//! `BarChart`, and a `table` widget natively -- covering every FC-DESKTOP-002/003 chart
//! type and the table demo, so this feasibility cycle standardises on it rather than
//! hand-rolled `Canvas` drawing or a general-purpose plotting-library adapter (see issue
//! #28's discussion for the comparison against `gpui-d3rs` and `ruviz-gpui`).
//!
//! Multiple screens are switched via `gpui-component`'s own `TabBar`, following the
//! precedent set by the TUI map: the second screen (doughnut) is where tab navigation
//! was introduced there too.
//!
//! The divergent (diverging bar) chart is the one type `gpui-component` doesn't cover --
//! its `BarChart` always anchors bars to the bottom of the plot area rather than a
//! zero-value baseline, so negative values don't diverge from a centre line. No crate for
//! this chart type was found for any framework surveyed (see
//! `docs/research/desktop-gui-frameworks.md`), so it's hand-rolled directly on `gpui`'s own
//! `canvas()`/`paint_quad`, the same fallback the TUI cycle used for its own divergent
//! chart (ADR-0002).

use gpui::{
    App, Application, Bounds, Context, Pixels, SharedString, Window, WindowBounds, WindowOptions,
    canvas, div, fill, point, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme,
    chart::{CandlestickChart, LineChart, PieChart},
    tab::{Tab, TabBar},
};

/// A single dummy monthly-spend data point, standing in for a real transaction-total
/// report (FR.35).
struct SpendPoint {
    month: SharedString,
    amount: f64,
}

/// Dummy monthly spend series, for the line chart demo.
fn dummy_spend() -> Vec<SpendPoint> {
    [
        ("Jan", 120.0),
        ("Feb", 95.0),
        ("Mar", 140.0),
        ("Apr", 110.0),
        ("May", 180.0),
        ("Jun", 160.0),
        ("Jul", 200.0),
        ("Aug", 175.0),
        ("Sep", 220.0),
        ("Oct", 205.0),
        ("Nov", 240.0),
        ("Dec", 230.0),
    ]
    .into_iter()
    .map(|(month, amount)| SpendPoint {
        month: month.into(),
        amount,
    })
    .collect()
}

/// A single dummy category-spend data point, standing in for a real per-Category
/// report (FR.35).
struct CategorySpend {
    category: SharedString,
    amount: f64,
}

/// Dummy category spend breakdown, for the doughnut chart demo.
fn dummy_categories() -> Vec<CategorySpend> {
    [
        ("Rent", 1500.0),
        ("Groceries", 420.0),
        ("Utilities", 210.0),
        ("Transport", 180.0),
        ("Entertainment", 90.0),
    ]
    .into_iter()
    .map(|(category, amount)| CategorySpend {
        category: category.into(),
        amount,
    })
    .collect()
}

/// A single dummy daily OHLC price point, standing in for a real investment's price
/// history (see the "Personal Investors" future consideration in
/// `docs/product-requirements.md`).
struct Candle {
    date: SharedString,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

/// Dummy daily OHLC series -- a small deterministic random walk, for the candlestick
/// chart demo. Mirrors the TUI cycle's own candlestick demo data shape
/// (`crates/bins/bin-tui/src/screen/candlestick_chart.rs`).
fn dummy_candles() -> Vec<Candle> {
    const DAY_COUNT: usize = 30;

    let mut price = 100.0_f64;
    // A tiny xorshift PRNG: deterministic across runs/platforms, no `rand` dependency
    // needed for a feasibility demo.
    let mut seed: u64 = 42;
    let mut random_delta = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 400) as f64 / 100.0 - 2.0
    };

    (1..=DAY_COUNT)
        .map(|day| {
            let open = price;
            let close = (open + random_delta()).max(1.0);
            let high = open.max(close) + random_delta().abs();
            let low = (open.min(close) - random_delta().abs()).max(0.5);
            price = close;
            Candle {
                date: format!("Day {day}").into(),
                open,
                high,
                low,
                close,
            }
        })
        .collect()
}

/// One category's dummy budget-vs-actual variance: positive is under budget, negative is
/// over budget (see FR.37 in `docs/product-requirements.md`).
struct Variance {
    label: SharedString,
    amount: f64,
}

/// Dummy budget-variance data -- a mix of under- and over-budget categories, so both bar
/// directions are demonstrated. Same data as the TUI cycle's own divergent chart demo
/// (`crates/bins/bin-tui/src/screen/divergent_chart.rs`).
fn dummy_variances() -> Vec<Variance> {
    [
        ("Groceries", -45.0),
        ("Rent", 5.0),
        ("Transport", 30.0),
        ("Entertainment", -15.0),
        ("Utilities", 60.0),
    ]
    .into_iter()
    .map(|(label, amount)| Variance {
        label: label.into(),
        amount,
    })
    .collect()
}

/// Computes the diverging bar's rectangle within `bounds`, given `amount` and `max_abs`
/// (the largest `|amount|` across all rows, for consistent scaling across the chart).
/// Positive amounts extend right from the vertical centre line; negative amounts extend
/// left. Pure and display-independent so the scaling maths can be unit tested without a
/// live GPUI window.
fn divergent_bar_bounds(bounds: Bounds<Pixels>, amount: f64, max_abs: f64) -> Bounds<Pixels> {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let center_x = origin_x + width / 2.0;
    let half_width = width / 2.0;

    let fraction = if max_abs > 0.0 {
        (amount.abs() / max_abs) as f32
    } else {
        0.0
    };
    let bar_width = half_width * fraction;
    let bar_x = if amount >= 0.0 {
        center_x
    } else {
        center_x - bar_width
    };

    Bounds {
        origin: point(px(bar_x), px(origin_y)),
        size: size(px(bar_width), px(height)),
    }
}

/// Which demo screen is currently shown.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Line,
    Doughnut,
    Candlestick,
    Divergent,
}

impl Screen {
    const ALL: [Screen; 4] = [
        Screen::Line,
        Screen::Doughnut,
        Screen::Candlestick,
        Screen::Divergent,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|screen| *screen == self).unwrap()
    }

    fn label(self) -> &'static str {
        match self {
            Screen::Line => "Line Chart",
            Screen::Doughnut => "Doughnut Chart",
            Screen::Candlestick => "Candlestick Chart",
            Screen::Divergent => "Divergent Chart",
        }
    }
}

/// Root view: a `TabBar` switching between the chart demo screens.
struct DesktopApp {
    screen: Screen,
    spend: Vec<SpendPoint>,
    categories: Vec<CategorySpend>,
    candles: Vec<Candle>,
    variances: Vec<Variance>,
}

impl Render for DesktopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let screen = self.screen;
        let selected_index = screen.index();

        let chart_colors = [
            cx.theme().chart_1,
            cx.theme().chart_2,
            cx.theme().chart_3,
            cx.theme().chart_4,
            cx.theme().chart_5,
        ];

        let body = match screen {
            Screen::Line => div()
                .h(px(400.0))
                .w_full()
                .child(
                    LineChart::new(
                        self.spend
                            .iter()
                            .map(|point| (point.month.clone(), point.amount)),
                    )
                    .x(|(month, _)| month.clone())
                    .y(|(_, amount)| *amount)
                    .stroke(chart_colors[0])
                    .dot(),
                )
                .into_any_element(),
            Screen::Doughnut => div()
                .h(px(400.0))
                .w_full()
                .child(
                    PieChart::new(self.categories.iter().enumerate().map(|(index, point)| {
                        (
                            point.category.clone(),
                            point.amount,
                            chart_colors[index % chart_colors.len()],
                        )
                    }))
                    .value(|(_, amount, _)| *amount as f32)
                    .color(|(_, _, color)| *color)
                    .inner_radius(60.0)
                    .outer_radius(100.0),
                )
                .into_any_element(),
            Screen::Candlestick => div()
                .h(px(400.0))
                .w_full()
                .child(
                    CandlestickChart::new(self.candles.iter().map(|candle| {
                        (
                            candle.date.clone(),
                            candle.open,
                            candle.high,
                            candle.low,
                            candle.close,
                        )
                    }))
                    .x(|(date, ..)| date.clone())
                    .open(|(_, open, ..)| *open)
                    .high(|(_, _, high, ..)| *high)
                    .low(|(_, _, _, low, _)| *low)
                    .close(|(_, _, _, _, close)| *close)
                    .tick_margin(4),
                )
                .into_any_element(),
            Screen::Divergent => {
                let max_abs = self
                    .variances
                    .iter()
                    .map(|variance| variance.amount.abs())
                    .fold(0.0_f64, f64::max)
                    .max(1.0);
                let success = cx.theme().success;
                let danger = cx.theme().danger;
                let zero_line_color = cx.theme().border;

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .children(self.variances.iter().map(|variance| {
                        let amount = variance.amount;
                        let bar_color = if amount >= 0.0 { success } else { danger };
                        let sign = if amount >= 0.0 { "+" } else { "" };

                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .h(px(32.0))
                            .child(div().w(px(140.0)).child(variance.label.clone()))
                            .child(
                                div().flex_1().h_full().child(
                                    canvas(
                                        move |bounds, _window, _cx| {
                                            divergent_bar_bounds(bounds, amount, max_abs)
                                        },
                                        move |row_bounds, bar_bounds, window, _cx| {
                                            let center_x = f32::from(row_bounds.origin.x)
                                                + f32::from(row_bounds.size.width) / 2.0;
                                            window.paint_quad(fill(
                                                Bounds {
                                                    origin: point(
                                                        px(center_x - 1.0),
                                                        row_bounds.origin.y,
                                                    ),
                                                    size: size(px(2.0), row_bounds.size.height),
                                                },
                                                zero_line_color,
                                            ));
                                            window.paint_quad(fill(bar_bounds, bar_color));
                                        },
                                    )
                                    .size_full(),
                                ),
                            )
                            .child(
                                div()
                                    .w(px(60.0))
                                    .text_color(bar_color)
                                    .child(format!("{sign}{amount:.0}")),
                            )
                    }))
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                TabBar::new("screens")
                    .children(
                        Screen::ALL
                            .iter()
                            .map(|screen| Tab::new().label(screen.label())),
                    )
                    .selected_index(selected_index)
                    .on_click(move |index, _window, cx| {
                        let next = Screen::ALL[*index];
                        entity.update(cx, |this, cx| {
                            this.screen = next;
                            cx.notify();
                        });
                    }),
            )
            .child(div().flex_1().p_4().child(body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(width: f32, height: f32) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(10.0), px(20.0)),
            size: size(px(width), px(height)),
        }
    }

    #[test]
    fn zero_amount_has_no_width() {
        let bar = divergent_bar_bounds(bounds(400.0, 32.0), 0.0, 60.0);
        assert_eq!(f32::from(bar.size.width), 0.0);
    }

    #[test]
    fn positive_amount_extends_right_from_centre() {
        let bounds = bounds(400.0, 32.0);
        let bar = divergent_bar_bounds(bounds, 30.0, 60.0);
        let center_x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;

        assert_eq!(f32::from(bar.origin.x), center_x);
        // Half the max, so half of the half-width available on the positive side.
        assert_eq!(
            f32::from(bar.size.width),
            f32::from(bounds.size.width) / 4.0
        );
    }

    #[test]
    fn negative_amount_extends_left_from_centre() {
        let bounds = bounds(400.0, 32.0);
        let bar = divergent_bar_bounds(bounds, -60.0, 60.0);
        let center_x = f32::from(bounds.origin.x) + f32::from(bounds.size.width) / 2.0;
        let half_width = f32::from(bounds.size.width) / 2.0;

        // The largest magnitude fills the whole half-width, ending exactly at centre.
        assert_eq!(f32::from(bar.size.width), half_width);
        assert_eq!(f32::from(bar.origin.x), center_x - half_width);
    }

    #[test]
    fn zero_max_abs_produces_no_bar() {
        let bar = divergent_bar_bounds(bounds(400.0, 32.0), 0.0, 0.0);
        assert_eq!(f32::from(bar.size.width), 0.0);
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(800.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                cx.new(|_cx| DesktopApp {
                    screen: Screen::Line,
                    spend: dummy_spend(),
                    categories: dummy_categories(),
                    candles: dummy_candles(),
                    variances: dummy_variances(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
