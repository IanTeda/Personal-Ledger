use gpui::{App, Context, SharedString, Window, div, prelude::*, px};
use lib_core::UnitId;

use super::GeneralPage;
use crate::form::fields::{field_label, field_value, radio_dot};
use crate::theme::color;

/// One ledger's worth of "THIS LEDGER" summary figures -- representative content matching the
/// mockup's own row order (accounts, transactions, units, institutions), not real
/// `lib_database` counts.
fn ledger_summary() -> [(String, &'static str); 4] {
    [
        (crate::msg::desktop_settings_general_summary_accounts(), "7"),
        (
            crate::msg::desktop_settings_general_summary_transactions(),
            "680",
        ),
        (crate::msg::desktop_settings_general_summary_units(), "3"),
        (
            crate::msg::desktop_settings_general_summary_institutions(),
            "7",
        ),
    ]
}

impl Render for GeneralPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let options = self.unit_options(cx);
        let base = self.base_unit(cx);
        let page_focused = self.page_focused;
        let cx: &App = cx;
        div()
            .flex()
            .gap(px(40.0))
            .child(field_column(
                &options,
                base,
                page_focused,
                move |id, cx| entity.update(cx, |page, cx| page.click_base_unit(id, cx)),
                cx,
            ))
            .child(summary_panel(cx))
    }
}

/// The 320px field column: Ledger name, Owner, Financial year starts, Base unit -- in the
/// mockup's own order.
fn field_column(
    options: &[(UnitId, String, String)],
    base: UnitId,
    page_focused: bool,
    on_pick: impl Fn(UnitId, &mut App) + Clone + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .w(px(320.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(field(
            crate::msg::desktop_settings_general_name(),
            "Personal Ledger",
            false,
            cx,
        ))
        .child(field(
            crate::msg::desktop_settings_general_owner(),
            "alex@teda.id.au",
            false,
            cx,
        ))
        .child(field(
            crate::msg::desktop_settings_general_financial_year(),
            "july",
            true,
            cx,
        ))
        .child(base_unit_select(options, base, page_focused, on_pick, cx))
}

/// The *Base unit* select: every Unit as a radio row, the base one checked. The keyboard moves it
/// while the page has focus, which draws the border in the accent.
fn base_unit_select(
    options: &[(UnitId, String, String)],
    base: UnitId,
    page_focused: bool,
    on_pick: impl Fn(UnitId, &mut App) + Clone + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .child(field_label(crate::msg::desktop_settings_general_base_unit()))
        .child(
            div()
                .debug_selector(|| "settings-general-base-unit".to_string())
                .w_full()
                .flex()
                .flex_col()
                .border_1()
                .border_color(if page_focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .bg(color::background(cx))
                .children(options.iter().map(|(id, code, name)| {
                    let (id, on_pick) = (*id, on_pick.clone());
                    let selector = format!("settings-general-base-unit-{code}");
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
                        .on_click(move |_event, _window, cx| on_pick(id, cx))
                        .child(radio_dot(id == base, cx))
                        .child(crate::msg::desktop_settings_general_base_unit_option(
                            code, name,
                        ))
                })),
        )
}

fn field(label: String, value: &'static str, select_style: bool, cx: &App) -> impl IntoElement {
    div()
        .child(field_label(label))
        .child(field_value(value, select_style, cx))
}

/// The right-hand "THIS LEDGER" panel: a bordered figure table, then a note explaining General's
/// own Preference-vs-Configuration scope (mirrors the section's own "ledger identity" scope
/// note).
fn summary_panel(cx: &App) -> impl IntoElement {
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
                .child(lib_locale::format::upper(
                    &crate::msg::desktop_settings_general_summary_title(),
                )),
        )
        .child(
            div()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .flex()
                .flex_col()
                .children({
                    let summary = ledger_summary();
                    let last = summary.len() - 1;
                    summary
                        .into_iter()
                        .enumerate()
                        .map(move |(index, (label, value))| {
                            summary_row(label, value, index == last, cx)
                        })
                }),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_general_note()),
        )
}

fn summary_row(label: String, value: &'static str, last: bool, cx: &App) -> impl IntoElement {
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
