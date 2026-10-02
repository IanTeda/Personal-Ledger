//! The 4a footer bar (`docs/ux/desktop/design_handoff_transactions/README.md`): `N of M … transactions shown` on the
//! left, and on the right the RUNNING TOTAL -- the figure the last row's running column ends on --
//! or `mixed units` when the visible rows span more than one Unit and no sum exists (the map's
//! no-cross-Unit rule).

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::{
    theme::color,
    transaction_chips::{Footer, FooterTotal},
};

/// `padding:11px 28px; border-top:2px solid rgba(32,30,29,.38); background:#eae9e9`.
pub fn render(footer: &Footer, cx: &App) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .px(px(28.0))
        .py(px(11.0))
        .border_t(px(2.0))
        .border_color(color::structural_rule(cx))
        .bg(color::chrome(cx))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(footer.label()),
        )
        .child(total(&footer.total, cx))
        .into_any_element()
}

/// The label (11px, tertiary) and the figure (20px, 800; `#ae1800` when negative), with the Unit's
/// code after a single-Unit figure.
fn total(total: &FooterTotal, cx: &App) -> impl IntoElement {
    let row = div().flex().items_baseline().gap(px(10.0)).child(
        div()
            .text_size(px(11.0))
            .text_color(color::faint_text(cx))
            .child(lib_locale::format::upper(
                &crate::msg::desktop_transactions_total_label(),
            )),
    );
    let figure = |text: String, text_color| {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(20.0))
            .text_color(text_color)
            .whitespace_nowrap()
            .child(text)
    };
    match total {
        FooterTotal::Single {
            unit,
            negative,
            text,
        } => row
            .child(figure(
                text.clone(),
                if *negative {
                    color::negative_text(cx)
                } else {
                    color::foreground(cx)
                },
            ))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(color::faint_text(cx))
                    .child(unit.clone()),
            ),
        FooterTotal::Mixed => row.child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(13.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_transactions_total_mixed()),
        ),
        FooterTotal::Empty => row.child(figure("\u{2014}".to_string(), color::faint_text(cx))),
    }
}
