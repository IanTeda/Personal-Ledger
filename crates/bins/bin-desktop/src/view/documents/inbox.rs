//! The Inbox (4b): the list pane's column header, rows, footnote and drop target, and the detail
//! pane's extracted facts, Suggested Link card, other candidates and key hints. A child of the
//! Documents page so it shares the Library's buttons, glyph and preview. Pure drawing: `Shell`
//! owns the focus, the filing actions and every key.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_locale::format::upper;

use super::{
    DETAIL_WIDTH, DocumentsPageProps, OnPlainClick, fact_row, file_glyph, preview, primary_button,
    secondary_button,
};
use crate::{
    documents::Signals,
    documents::model::{CandidateView, InboxDetailView, InboxRowView, InboxState},
    theme::color,
};

/// The list pane below the header: column header, rows (or the empty state), footnote and drop
/// target.
pub fn list_body(props: &DocumentsPageProps, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(column_header(cx))
        .child(rows(props, cx))
        .child(footnote(props, cx))
        .child(drop_target(cx))
        .into_any_element()
}

/// The header's right-hand buttons: **Watched folder…** and **Accept all strong matches · n**.
pub fn header_buttons(props: &DocumentsPageProps, cx: &App) -> [AnyElement; 2] {
    let strong = props.inbox_strong;
    let accept_all: AnyElement = if strong > 0 {
        primary_button(
            "documents-accept-all",
            crate::msg::desktop_documents_accept_all(&strong.to_string()),
            props.on_accept_all_click.clone(),
            cx,
        )
        .into_any_element()
    } else {
        // Disabled: dimmed, with no click handler.
        div()
            .id("documents-accept-all")
            .flex_none()
            .flex()
            .items_center()
            .h(px(36.0))
            .px(px(14.0))
            .bg(color::foreground(cx))
            .text_color(color::selection_text(cx))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .whitespace_nowrap()
            .opacity(0.45)
            .child(crate::msg::desktop_documents_accept_all("0"))
            .into_any_element()
    };
    [
        secondary_button(
            "documents-watched-folder",
            crate::msg::desktop_documents_watched_folder(),
            props.on_watched_click.clone(),
            cx,
        )
        .into_any_element(),
        accept_all,
    ]
}

fn column_header(cx: &App) -> impl IntoElement {
    let label = |text: String| {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(10.0))
            .text_color(color::muted(cx))
            .child(upper(&text))
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(8.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .child(
            div()
                .w(px(252.0))
                .flex_none()
                .child(label(crate::msg::desktop_documents_inbox_col_file())),
        )
        .child(
            div()
                .flex_1()
                .pl(px(26.0))
                .child(label(crate::msg::desktop_documents_inbox_col_link())),
        )
}

fn rows(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let body = div()
        .id("documents-inbox-rows")
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .track_scroll(&props.scroll);
    if props.inbox_rows.is_empty() {
        return body.child(
            div()
                .px(px(14.0))
                .py(px(24.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_documents_inbox_empty()),
        );
    }
    body.children(props.inbox_rows.iter().enumerate().map(|(index, row)| {
        inbox_row(
            row,
            index == props.inbox_selected && props.list_focused,
            index == props.inbox_selected,
            index,
            props,
            cx,
        )
    }))
}

fn inbox_row(
    row: &InboxRowView,
    focus_ring: bool,
    focused: bool,
    index: usize,
    props: &DocumentsPageProps,
    cx: &App,
) -> impl IntoElement {
    let on_click = props.on_row_click.clone();
    let on_accept = props.on_accept_row_click.clone();
    let ink = color::foreground(cx);
    let unreadable = row.state != InboxState::Link;
    let action_label = if unreadable {
        crate::msg::desktop_documents_file_button()
    } else {
        crate::msg::desktop_documents_accept()
    };
    let summary_color = match row.state {
        InboxState::Unreadable => color::negative_text(cx),
        _ => ink,
    };
    div()
        .id(("documents-inbox-row", index))
        .debug_selector(|| format!("documents-inbox-row-{index}"))
        .relative()
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(9.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .when(focused, |this| this.bg(color::background(cx)))
        .when(row.skipped, |this| this.opacity(0.55))
        .on_click(move |_event, window, cx| on_click(index, window, cx))
        .child(file_glyph(row.extension, ink))
        .child(
            div()
                .w(px(210.0))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(12.5))
                        .truncate()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::faint_text(cx))
                        .truncate()
                        .child(row.source.clone()),
                ),
        )
        .child(
            div()
                .flex_none()
                .text_color(color::faint_text(cx))
                .child("\u{2192}"),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(12.5))
                        .text_color(summary_color)
                        .truncate()
                        .child(row.summary.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .text_size(px(11.0))
                        .text_color(color::muted(cx))
                        .children(row.signals.map(|signals| meter(signals, cx)))
                        .child(div().min_w(px(0.0)).truncate().child(row.hint.clone())),
                ),
        )
        .child(
            div()
                .id(("documents-inbox-accept", index))
                .debug_selector(|| format!("documents-inbox-accept-{index}"))
                .cursor_pointer()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(30.0))
                .px(px(12.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(focused, |this| {
                    this.bg(ink).text_color(color::selection_text(cx))
                })
                .when(!focused, |this| {
                    this.border_1().border_color(color::border(cx))
                })
                .on_click(move |_event, window, cx| {
                    cx.stop_propagation();
                    on_accept(index, window, cx);
                })
                .child(action_label)
                .when(focused, |this| {
                    this.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(color::selection_muted(cx))
                            .child("y"),
                    )
                }),
        )
        // Outline focus, not the Library's inverted fill: a 2px inset ring over the row.
        .when(focus_ring, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .border_2()
                    .border_color(ink),
            )
        })
}

/// The match meter: three 6×6 squares, filled for each Signal that agrees (amount, date, payee).
pub fn meter(signals: Signals, cx: &App) -> impl IntoElement {
    let square = |agrees: bool| {
        div()
            .size(px(6.0))
            .when(agrees, |this| this.bg(color::foreground(cx)))
            .when(!agrees, |this| {
                this.border_1().border_color(color::faint_text(cx))
            })
    };
    div()
        .flex_none()
        .flex()
        .gap(px(2.0))
        .child(square(signals.amount))
        .child(square(signals.date))
        .child(square(signals.payee))
}

fn footnote(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    // Rows below the last one in view, read from the list's scroll state of the previous frame.
    let below = props
        .inbox_rows
        .len()
        .saturating_sub(props.scroll.bottom_item() + 1);
    let note = crate::msg::desktop_documents_inbox_footnote();
    let text = if below > 0 && !props.inbox_rows.is_empty() {
        format!(
            "{} · {note}",
            crate::msg::desktop_documents_inbox_below(i64::try_from(below).unwrap_or(i64::MAX))
        )
    } else {
        note
    };
    div()
        .flex_none()
        .px(px(14.0))
        .py(px(8.0))
        .text_size(px(11.0))
        .text_color(color::faint_text(cx))
        .child(text)
}

/// The dashed drop target. Drawn as the handoff has it; the drop itself is handled by the Shell's root.
fn drop_target(cx: &App) -> impl IntoElement {
    div()
        .flex_none()
        .mx(px(14.0))
        .mb(px(14.0))
        .p(px(16.0))
        .border_2()
        .border_dashed()
        .border_color(color::faint_text(cx))
        .flex()
        .items_center()
        .gap(px(10.0))
        .text_size(px(12.5))
        .text_color(color::muted(cx))
        .child(div().child("\u{2193}"))
        .child(crate::msg::desktop_documents_drop_target())
}

// ---------------------------------------------------------------------------------------------
// Detail pane
// ---------------------------------------------------------------------------------------------

pub fn detail_pane(props: &DocumentsPageProps, cx: &App) -> AnyElement {
    let pane = div()
        .w(DETAIL_WIDTH)
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx));
    let Some(detail) = props.inbox_detail.as_ref() else {
        return pane.into_any_element();
    };
    pane.child(preview(detail.extension, cx))
        .child(
            div()
                .id("documents-inbox-detail-body")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .px(px(16.0))
                .py(px(12.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_size(px(15.0))
                                .line_height(gpui::relative(1.25))
                                .child(detail.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(color::muted(cx))
                                .child(detail.meta.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .children(detail.facts.iter().map(|fact| fact_row(fact, cx))),
                )
                .children(
                    detail
                        .suggested
                        .as_ref()
                        .map(|card| suggested_card(card, cx)),
                )
                .child(other_candidates(detail, props, cx)),
        )
        .child(key_hints(props, cx))
        .child(footer(detail, props, cx))
        .into_any_element()
}

fn suggested_card(card: &CandidateView, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .p(px(10.0))
        .border_2()
        .border_color(color::foreground(cx))
        .bg(color::background(cx))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(9.5))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_documents_suggested_link())),
        )
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(card.line.clone()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(card.sub.clone()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .text_size(px(11.0))
                .text_color(color::muted(cx))
                .child(meter(card.signals, cx))
                .child(card.signals_text.clone()),
        )
}

fn other_candidates(
    detail: &InboxDetailView,
    props: &DocumentsPageProps,
    cx: &App,
) -> impl IntoElement {
    if detail.state != InboxState::Link {
        return div().into_any_element();
    }
    if detail.others.is_empty() {
        return div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(detail.others_none.clone())
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_documents_other_candidates())),
        )
        .children(detail.others.iter().enumerate().map(|(index, other)| {
            let on_accept = props.on_accept_candidate_click.clone();
            let transaction_id = other.transaction_id;
            div()
                .id(("documents-inbox-other", index))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .py(px(6.0))
                .border_1()
                .border_color(color::border(cx))
                .bg(color::background(cx))
                .text_size(px(11.5))
                .on_click(move |_event, window, cx| on_accept(transaction_id, window, cx))
                .child(meter(other.signals, cx))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .child(other.line.clone()),
                )
        }))
        .into_any_element()
}

/// The strip under the detail body: each key in bold, then its action.
fn key_hints(props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .flex_wrap()
        .gap_x(px(8.0))
        .px(px(16.0))
        .py(px(8.0))
        .border_t(px(1.0))
        .border_color(color::border(cx))
        .text_size(px(11.0))
        .text_color(color::muted(cx))
        .children(props.inbox_hints.iter().map(|(keys, action)| {
            div()
                .flex()
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::foreground(cx))
                        .child(SharedString::from(*keys)),
                )
                .child(action.clone())
        }))
}

fn footer(detail: &InboxDetailView, props: &DocumentsPageProps, cx: &App) -> impl IntoElement {
    let readable = detail.state == InboxState::Link;
    let (primary_label, primary_click): (String, OnPlainClick) = if readable {
        (
            crate::msg::desktop_documents_accept_next(),
            props.on_accept_next_click.clone(),
        )
    } else {
        (
            crate::msg::desktop_documents_file_button(),
            props.on_link_elsewhere_click.clone(),
        )
    };
    let (secondary_label, secondary_click): (String, OnPlainClick) = if readable {
        (
            crate::msg::desktop_documents_link_elsewhere(),
            props.on_link_elsewhere_click.clone(),
        )
    } else {
        (
            crate::msg::desktop_documents_skip(),
            props.on_skip_click.clone(),
        )
    };
    let hover = color::muted(cx);
    div()
        .flex_none()
        .flex()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(12.0))
        .border_t(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(
            div()
                .id("documents-inbox-primary")
                .debug_selector(|| "documents-inbox-primary".to_string())
                .cursor_pointer()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .h(px(34.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| primary_click(window, cx))
                .child(primary_label),
        )
        .child(
            div()
                .id("documents-inbox-secondary")
                .debug_selector(|| "documents-inbox-secondary".to_string())
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .h(px(34.0))
                .px(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .on_click(move |_event, window, cx| secondary_click(window, cx))
                .child(secondary_label),
        )
}
