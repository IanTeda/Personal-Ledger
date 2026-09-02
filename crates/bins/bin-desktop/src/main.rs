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

use gpui::{
    App, Application, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
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

/// Which demo screen is currently shown.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Line,
    Doughnut,
    Candlestick,
}

impl Screen {
    const ALL: [Screen; 3] = [Screen::Line, Screen::Doughnut, Screen::Candlestick];

    fn index(self) -> usize {
        Self::ALL.iter().position(|screen| *screen == self).unwrap()
    }

    fn label(self) -> &'static str {
        match self {
            Screen::Line => "Line Chart",
            Screen::Doughnut => "Doughnut Chart",
            Screen::Candlestick => "Candlestick Chart",
        }
    }
}

/// Root view: a `TabBar` switching between the chart demo screens.
struct DesktopApp {
    screen: Screen,
    spend: Vec<SpendPoint>,
    categories: Vec<CategorySpend>,
    candles: Vec<Candle>,
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
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
