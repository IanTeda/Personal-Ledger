//! Draws the Unit prices view from `lib-units`' [`UnitPricesViewModel`] in place of the Units
//! page's tables: the title and its 12-month window over the PRICE figures, the WEEKLY CLOSE chart
//! (`gpui-component`'s candlestick chart, a close-only week drawn as a flat tick), then the WEEKLY
//! PRICES table, newest week first. A row click selects its week.

use gpui::{AnyElement, App, Entity, SharedString, div, prelude::*, px};
use gpui_component::chart::CandlestickChart;
use lib_units::presenter::{PriceRow, UnitPricesEvent, UnitPricesViewModel};

use super::UnitsPage;
use crate::theme::color;

/// The chart's height, its axis labels included.
const CHART_HEIGHT: gpui::Pixels = px(220.0);
/// Every price column's width; WEEK takes the rest.
const PRICE_WIDTH: gpui::Pixels = px(120.0);
/// Label every fourth week on the axis, so a year's labels don't overlap.
const AXIS_EVERY: usize = 4;

pub(super) fn prices_view(
    view: UnitPricesViewModel,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> AnyElement {
    let UnitPricesViewModel {
        title,
        facts,
        chart_heading,
        table_heading,
        window,
        chart,
        columns,
        rows,
        empty,
        ..
    } = view;
    let heading = |text: &str| {
        div()
            .mt(px(24.0))
            .mb(px(8.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(10.0))
            .text_color(color::faint_text(cx))
            .child(lib_locale::format::upper(text))
    };
    let header = div()
        .debug_selector(|| "unit-prices-header".to_string())
        .flex()
        .items_baseline()
        .justify_between()
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .child(title),
        )
        .child(
            div()
                .debug_selector(|| "unit-prices-window".to_string())
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(window),
        );
    let figures = div()
        .debug_selector(|| "unit-prices-facts".to_string())
        .mt(px(12.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .children(facts.into_iter().map(|fact| {
            div()
                .flex()
                .justify_between()
                .text_size(px(12.5))
                .child(div().text_color(color::muted(cx)).child(fact.label))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(fact.value),
                )
        }));
    let mut page = div()
        .debug_selector(|| "unit-prices".to_string())
        .flex()
        .flex_col()
        .child(header)
        .child(figures)
        .child(heading(&chart_heading));
    if let Some(empty) = empty {
        return page
            .child(
                div()
                    .debug_selector(|| "unit-prices-empty".to_string())
                    .text_color(color::muted(cx))
                    .child(empty),
            )
            .into_any_element();
    }
    page = page.child(
        div()
            .debug_selector(|| "unit-prices-chart".to_string())
            .h(CHART_HEIGHT)
            .child(
                CandlestickChart::new(chart)
                    .x(|week| SharedString::from(week.label.clone()))
                    .open(|week| week.open)
                    .high(|week| week.high)
                    .low(|week| week.low)
                    .close(|week| week.close)
                    .tick_margin(AXIS_EVERY),
            ),
    );
    let last = rows.len().saturating_sub(1);
    page.child(heading(&table_heading))
        .child(
            div()
                .debug_selector(|| "unit-prices-table".to_string())
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .child(
                    row_cells(
                        [
                            columns.week,
                            columns.open,
                            columns.high,
                            columns.low,
                            columns.close,
                            columns.change,
                        ],
                        None,
                    )
                    .px(px(16.0))
                    .py(px(10.0))
                    .bg(color::chrome(cx))
                    .border_b(px(1.0))
                    .border_color(color::border(cx))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(10.0))
                    .text_color(color::muted(cx)),
                )
                .children(
                    rows.into_iter()
                        .enumerate()
                        .map(|(index, row)| price_row(row, index, index == last, entity, cx)),
                ),
        )
        .into_any_element()
}

/// WEEK left, then the four prices and the change right-aligned in fixed columns, the change in
/// `change` when given.
fn row_cells(texts: [String; 6], change: Option<gpui::Rgba>) -> gpui::Div {
    let [week, rest @ ..] = texts;
    let last = rest.len() - 1;
    div()
        .flex()
        .items_center()
        .child(div().flex_1().child(week))
        .children(rest.into_iter().enumerate().map(|(index, text)| {
            div()
                .w(PRICE_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .when_some(change.filter(|_| index == last), |this, colour| {
                    this.text_color(colour)
                })
                .child(text)
        }))
}

fn price_row(
    row: PriceRow,
    index: usize,
    last: bool,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let id = SharedString::from(format!("unit-prices-row-{index}"));
    let selected = row.selected;
    let falling = row.falling;
    let week = row.week;
    let entity = entity.clone();
    // A fall is marked by its sign as well as the colour, so it never rests on colour alone.
    let change = (falling && !selected).then(|| color::accent_text(cx));
    let cells = row_cells(
        [
            row.label, row.open, row.high, row.low, row.close, row.change,
        ],
        change,
    );
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .px(px(16.0))
        .py(px(8.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(move |_event, _window, cx| {
            entity.update(cx, |page, cx| {
                page.send_prices(UnitPricesEvent::Select(week), cx);
            });
        })
        .child(cells)
}
