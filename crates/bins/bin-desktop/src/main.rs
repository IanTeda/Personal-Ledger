//! Personal Ledger desktop app -- feasibility-cycle demo.
//!
//! Demonstrates line chart rendering in `GPUI` (ADR-0007) against dummy data, per
//! FC-DESKTOP-002, using `gpui-component`'s `chart::LineChart`. `gpui-component` also
//! ships `PieChart` (with a real donut via `inner_radius`), `CandlestickChart`, `BarChart`
//! and a `table` widget natively -- covering every remaining FC-DESKTOP-002/003 chart type
//! and the table demo, so this feasibility cycle standardises on it rather than hand-rolled
//! `Canvas` drawing or a general-purpose plotting-library adapter (see issue #28's
//! discussion for the comparison against `gpui-d3rs` and `ruviz-gpui`).

use gpui::{
    App, Application, Bounds, Context, SharedString, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use gpui_component::{ActiveTheme, chart::LineChart};

/// A single dummy monthly-spend data point, standing in for a real transaction-total
/// report (FR.35).
struct SpendPoint {
    month: SharedString,
    amount: f64,
}

/// Dummy monthly spend series.
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

/// Root view for the line chart demo.
struct LineChartDemo {
    data: Vec<SpendPoint>,
}

impl Render for LineChartDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stroke = cx.theme().chart_1;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_4()
            .gap_2()
            .child(
                div()
                    .text_xl()
                    .child("Personal Ledger -- Line chart demo (dummy spend data, gpui-component)"),
            )
            .child(
                div().h(px(400.0)).w_full().child(
                    LineChart::new(
                        self.data
                            .iter()
                            .map(|point| (point.month.clone(), point.amount)),
                    )
                    .x(|(month, _)| month.clone())
                    .y(|(_, amount)| *amount)
                    .stroke(stroke)
                    .dot(),
                ),
            )
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
                cx.new(|_cx| LineChartDemo {
                    data: dummy_spend(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
