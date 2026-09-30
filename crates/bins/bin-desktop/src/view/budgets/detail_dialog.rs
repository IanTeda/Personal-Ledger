//! Renders the **Category detail** dialog (`docs/ux/desktop/Budgets_v2/limits-9a-9g.md`'s 9d) on
//! the shared `crate::dialog` chrome at the handoff's 600px: four stats, the Progress row's bar
//! with its caption, the largest Transactions behind Spent, the track record, and a footer that
//! hands off to Transactions already filtered.
//!
//! The Edit budget button and `e` key belong to 9e (#405).

use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use gpui::{AnyElement, App, div, prelude::*, px};
use lib_core::Money;

use crate::{budgets::CategoryDetail, dialog, format::amount, theme::color};

/// The dialog's width: the handoff's 9d.
pub const WIDTH: gpui::Pixels = px(600.0);

/// The Transactions listed before the `+ N more` row.
pub const LISTED: usize = 5;

/// A Transaction row, worded for the list.
pub struct LineRow {
    pub date: String,
    pub payee: String,
    pub account: String,
    pub amount: String,
}

pub struct DetailDialogProps<'a> {
    pub title: String,
    pub detail: &'a CategoryDetail,
    pub lines: Vec<LineRow>,
    /// The `+ N more · amount` row, when there are more than [`LISTED`] lines.
    pub more: Option<String>,
    /// Index into `lines` of the `j`/`k` cursor.
    pub selected: usize,
    pub track: String,
    pub rollover_note: Option<String>,
    pub bills: String,
    pub on_close: dialog::OnClick,
    pub on_open_transactions: dialog::OnClick,
}

fn text(money: &Money) -> String {
    amount(money).1
}

fn stat(label: String, value: String, negative: bool, cx: &App) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::muted(cx))
                .child(lib_locale::format::upper(&label)),
        )
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if negative {
                    color::negative_text(cx)
                } else {
                    color::foreground(cx)
                })
                .child(value),
        )
}

fn spent_percent(detail: &CategoryDetail) -> Option<i64> {
    let budget = detail.figures.budget.as_ref().filter(|b| !b.0.is_zero())?;
    let ratio: BigDecimal = detail.figures.spent.0.clone() * 100 / budget.0.clone();
    ratio.round(0).to_i64()
}

pub fn render(props: DetailDialogProps<'_>, cx: &App) -> AnyElement {
    let DetailDialogProps {
        title,
        detail,
        lines,
        more,
        selected,
        track,
        rollover_note,
        bills,
        on_close,
        on_open_transactions,
    } = props;
    let figures = &detail.figures;
    let budget = figures
        .budget
        .as_ref()
        .map_or_else(|| crate::transaction_rows::EMPTY_CELL.to_string(), text);
    // OVER when Spent has passed the budget, otherwise what is LEFT.
    let (last_label, last_value, over) = if figures.over {
        let over_by = figures.budget.as_ref().map_or_else(
            || figures.spent.0.clone(),
            |b| figures.spent.0.clone() - b.0.clone(),
        );
        (
            crate::msg::desktop_budgets_detail_stat_over(),
            text(&Money(over_by)),
            true,
        )
    } else {
        let left = figures.left.as_ref();
        (
            crate::msg::desktop_budgets_detail_stat_left(),
            left.map_or_else(|| crate::transaction_rows::EMPTY_CELL.to_string(), text),
            left.is_some_and(|l| l.0.is_negative()),
        )
    };
    let caption = spent_percent(detail).map_or_else(
        || {
            crate::msg::desktop_budgets_detail_caption_unbudgeted(
                &detail.elapsed.percent.to_string(),
            )
        },
        |spent| {
            crate::msg::desktop_budgets_detail_caption(
                &spent.to_string(),
                &detail.elapsed.percent.to_string(),
            )
        },
    );

    let stats = div()
        .flex()
        .gap(px(16.0))
        .child(stat(
            crate::msg::desktop_budgets_stat_budgeted(),
            budget,
            false,
            cx,
        ))
        .child(stat(
            crate::msg::desktop_budgets_stat_spent(),
            text(&figures.spent),
            false,
            cx,
        ))
        .child(stat(
            crate::msg::desktop_budgets_stat_known(),
            text(&figures.known),
            false,
            cx,
        ))
        .child(stat(last_label, last_value, over, cx));

    let bar = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(super::progress::bar(
            figures,
            detail.elapsed.percent,
            false,
            cx,
        ))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(caption),
        );

    let header_line = |label: String| {
        div()
            .pb(px(6.0))
            .border_b(px(1.0))
            .border_color(color::hairline(cx))
            .text_size(px(10.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::muted(cx))
            .child(lib_locale::format::upper(&label))
    };
    let count = i64::try_from(detail.lines.len()).unwrap_or(i64::MAX);
    let list = div()
        .flex()
        .flex_col()
        .child(header_line(if count == 0 {
            crate::msg::desktop_budgets_detail_transactions_none()
        } else {
            crate::msg::desktop_budgets_detail_transactions(count)
        }))
        .children(lines.into_iter().enumerate().map(|(index, line)| {
            let active = index == selected;
            div()
                .flex()
                .gap(px(12.0))
                .py(px(6.0))
                .border_b(px(1.0))
                .border_color(color::hairline(cx))
                .text_size(px(12.5))
                .when(active, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .child(div().w(px(64.0)).flex_none().child(line.date))
                .child(div().flex_1().child(line.payee))
                .child(div().w(px(150.0)).flex_none().child(line.account))
                .child(
                    div()
                        .w(px(90.0))
                        .flex_none()
                        .text_right()
                        .child(line.amount),
                )
        }))
        .children(more.map(|row| {
            div()
                .py(px(6.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(row)
        }))
        .child(
            div()
                .pt(px(6.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(bills),
        );

    let track = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .text_size(px(12.5))
        .child(track)
        .children(rollover_note.map(|note| div().text_color(color::muted(cx)).child(note)));

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(vec![
            stats.into_any_element(),
            bar.into_any_element(),
            list.into_any_element(),
            track.into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                div()
                    .id("budgets-detail-transactions")
                    .cursor_pointer()
                    .py(px(8.0))
                    .px(px(16.0))
                    .mr_auto()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::foreground(cx))
                    .on_click(move |_event, window, cx| on_open_transactions(window, cx))
                    .child(crate::msg::desktop_budgets_detail_open_transactions())
                    .into_any_element(),
                dialog::confirm_button(
                    "budgets-detail-close",
                    crate::msg::desktop_budgets_detail_close(),
                    true,
                    false,
                    on_close,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
