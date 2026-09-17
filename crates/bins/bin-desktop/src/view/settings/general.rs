//! The **General** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state", first in
//! scroll order): ledger identity fields plus a "THIS LEDGER" summary panel. Dummy values
//! throughout, matching the mockup's own -- no real `lib_database::Preferences` wiring in this
//! map (see the Desktop Settings Surface map's Destination).

use gpui::{AnyElement, div, prelude::*, px};

use crate::theme::color;

use super::{field_label, field_value};

/// One ledger's worth of "THIS LEDGER" summary figures -- representative content matching the
/// mockup's own row order (accounts, transactions, units, institutions), not real
/// `lib_database` counts.
const LEDGER_SUMMARY: &[(&str, &str)] = &[
    ("accounts", "7"),
    ("transactions", "680"),
    ("units", "3"),
    ("institutions", "7"),
];

pub fn render() -> AnyElement {
    div()
        .flex()
        .gap(px(40.0))
        .child(field_column())
        .child(summary_panel())
        .into_any_element()
}

/// The 320px field column: Ledger name, Owner, Financial year starts, Base unit -- in the
/// mockup's own order.
fn field_column() -> impl IntoElement {
    div()
        .w(px(320.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(field("Ledger name", "Personal Ledger", false))
        .child(field("Owner", "alex@teda.id.au", false))
        .child(field("Financial year starts", "july", true))
        .child(field("Base unit", "aud \u{2014} Australian Dollar", true))
}

fn field(label: &'static str, value: &'static str, select_style: bool) -> impl IntoElement {
    div()
        .child(field_label(label))
        .child(field_value(value, select_style))
}

/// The right-hand "THIS LEDGER" panel: a bordered figure table, then a note explaining General's
/// own Preference-vs-Configuration scope (mirrors the section's own "ledger identity" scope
/// note).
fn summary_panel() -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(400.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::INK_TERTIARY)
                .mb(px(10.0))
                .child("THIS LEDGER"),
        )
        .child(
            div()
                .border_1()
                .border_color(color::BORDER)
                .bg(color::CHROME)
                .flex()
                .flex_col()
                .children(
                    LEDGER_SUMMARY
                        .iter()
                        .enumerate()
                        .map(|(index, (label, value))| {
                            summary_row(label, value, index == LEDGER_SUMMARY.len() - 1)
                        }),
                ),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::INK_SECONDARY)
                .child(
                    "General settings are ledger-scoped Preferences \u{2014} they travel with \
                     the ledger as Change Sets. Client-only Configuration lives under Display.",
                ),
        )
}

fn summary_row(label: &'static str, value: &'static str, last: bool) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .px(px(12.0))
        .py(px(10.0))
        .text_size(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
        })
        .child(div().text_color(color::INK_SECONDARY).child(label))
        .child(div().font_weight(gpui::FontWeight::EXTRA_BOLD).child(value))
}
