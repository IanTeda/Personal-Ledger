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
    chart::{LineChart, PieChart},
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

/// Which demo screen is currently shown.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Line,
    Doughnut,
}

/// Root view: a `TabBar` switching between the chart demo screens.
struct DesktopApp {
    screen: Screen,
    spend: Vec<SpendPoint>,
    categories: Vec<CategorySpend>,
}

impl Render for DesktopApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let screen = self.screen;
        let selected_index = match screen {
            Screen::Line => 0,
            Screen::Doughnut => 1,
        };

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
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                TabBar::new("screens")
                    .child(Tab::new().label("Line Chart"))
                    .child(Tab::new().label("Doughnut Chart"))
                    .selected_index(selected_index)
                    .on_click(move |index, _window, cx| {
                        let next = if *index == 0 {
                            Screen::Line
                        } else {
                            Screen::Doughnut
                        };
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
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
