//! The **General** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state", first in
//! scroll order): ledger identity fields plus a "THIS LEDGER" summary panel. Dummy values
//! throughout, matching the mockup's own -- no real `lib_database::Preferences` wiring in this
//! map (see the Desktop Settings Surface map's Destination).

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::theme::color;

use super::{field_label, field_value};

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

pub fn render(cx: &App) -> AnyElement {
    div()
        .flex()
        .gap(px(40.0))
        .child(field_column(cx))
        .child(summary_panel(cx))
        .into_any_element()
}

/// The 320px field column: Ledger name, Owner, Financial year starts, Base unit -- in the
/// mockup's own order.
fn field_column(cx: &App) -> impl IntoElement {
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
        .child(field(
            crate::msg::desktop_settings_general_base_unit(),
            "aud \u{2014} Australian Dollar",
            true,
            cx,
        ))
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
