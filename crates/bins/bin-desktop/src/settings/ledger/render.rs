//! Settings › Ledger drawn from its view model: the field column (Ledger name, Owner, Financial
//! year starts, Base unit) beside the THIS LEDGER summary. Each Base unit option's click is a
//! Presenter event through [`PageClicks`].

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_settings::LedgerEvent;
use lib_settings::presenter::{BaseUnitView, LedgerViewModel};

use super::LedgerPage;
use crate::form::fields::{field_label, field_value, radio_dot};
use crate::shell::view::page_clicks::PageClicks;
use crate::theme::color;

/// The page drawn from `model`.
pub(super) fn page(
    model: &LedgerViewModel,
    clicks: &PageClicks<LedgerPage>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .gap(px(40.0))
        .child(field_column(model, clicks, cx))
        .child(summary_panel(model, cx))
        .into_any_element()
}

/// The 320px field column: Ledger name, Owner, Financial year starts, Base unit -- in the
/// mockup's own order.
fn field_column(
    model: &LedgerViewModel,
    clicks: &PageClicks<LedgerPage>,
    cx: &App,
) -> impl IntoElement {
    let last = model.fields.len().saturating_sub(1);
    div()
        .w(px(320.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .children(
            model
                .fields
                .iter()
                .enumerate()
                .map(|(index, (label, value))| {
                    field(
                        label.clone(),
                        value.clone(),
                        model.financial_year_select && index == last,
                        cx,
                    )
                }),
        )
        .child(base_unit_select(&model.base_unit, clicks, cx))
}

/// The *Base unit* select: every Unit as a radio row, the base one checked. The keyboard moves it
/// while the page has focus, which draws the border in the accent.
fn base_unit_select(
    select: &BaseUnitView,
    clicks: &PageClicks<LedgerPage>,
    cx: &App,
) -> impl IntoElement {
    div().child(field_label(select.label.clone())).child(
        div()
            .debug_selector(|| "settings-ledger-base-unit".to_string())
            .w_full()
            .flex()
            .flex_col()
            .border_1()
            .border_color(if select.focused {
                color::accent(cx)
            } else {
                color::border(cx)
            })
            .bg(color::background(cx))
            .children(select.options.iter().map(|option| {
                let selector = format!("settings-ledger-base-unit-{}", option.code);
                div()
                    .debug_selector({
                        let selector = selector.clone();
                        move || selector.clone()
                    })
                    .id(SharedString::from(selector))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(8.0))
                    .px(px(10.0))
                    .text_size(px(13.0))
                    .on_click(clicks.send(LedgerEvent::ChooseBaseUnit(option.id)))
                    .child(radio_dot(select.chosen == Some(option.id), cx))
                    .child(option.label.clone())
            })),
    )
}

fn field(label: String, value: String, select_style: bool, cx: &App) -> impl IntoElement {
    div()
        .child(field_label(label))
        .child(field_value(value, select_style, cx))
}

/// The right-hand "THIS LEDGER" panel: a bordered figure table, then a note explaining the page's
/// own Preference-vs-Configuration scope (mirrors the section's own "ledger identity" scope
/// note).
fn summary_panel(model: &LedgerViewModel, cx: &App) -> impl IntoElement {
    let last = model.summary.len().saturating_sub(1);
    div()
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(400.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .mb(px(10.0))
                .child(lib_locale::format::upper(&model.summary_title)),
        )
        .child(
            div()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .flex()
                .flex_col()
                .children(
                    model
                        .summary
                        .iter()
                        .enumerate()
                        .map(|(index, (label, value))| {
                            summary_row(label.clone(), value.clone(), index == last, cx)
                        }),
                ),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(model.note.clone()),
        )
}

fn summary_row(label: String, value: String, last: bool, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .px(px(12.0))
        .py(px(10.0))
        .text_size(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(div().text_color(color::muted(cx)).child(label))
        .child(div().font_weight(gpui::FontWeight::EXTRA_BOLD).child(value))
}
