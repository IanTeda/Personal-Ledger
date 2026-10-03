//! Renders the **Fill from…** dialog (`docs/ux/desktop/14-budgets-v2/README.md`'s 9f) on the
//! shared `crate::dialog` chrome at the handoff's 560px: the two sources as radio rows, each with
//! the Total budgeted it would give the month, then the chosen source's diff against the plan.
//! "The plan" is the baseline rather than a source (#386).

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::Money;
use lib_locale::format::upper;

use crate::{
    budgets::{FillPreview, FillSource},
    dialog,
    format::{amount, signed_amount},
    theme::color,
    transaction_rows::EMPTY_CELL,
};

/// The dialog's width: the handoff's 9f.
pub const WIDTH: gpui::Pixels = px(560.0);

pub type OnSourceClick = Rc<dyn Fn(FillSource, &mut Window, &mut App)>;

/// A source's radio row, already worded.
pub struct SourceRow {
    pub source: FillSource,
    pub title: String,
    pub detail: String,
    /// The target month's Total budgeted if this source fills it.
    pub total: String,
}

pub struct FillDialogProps<'a> {
    /// The target month, already formatted.
    pub month: String,
    pub sources: Vec<SourceRow>,
    /// The chosen source's preview.
    pub preview: &'a FillPreview,
    /// Names for `preview.changes`, in its order, and for `preview.kept`.
    pub change_names: Vec<String>,
    pub kept_names: Vec<String>,
    pub on_source_click: OnSourceClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

fn money(value: &Money) -> String {
    amount(value).1
}

/// A bordered radio row; the chosen one gets the accent border and tint.
fn source_row(row: SourceRow, selected: bool, on_click: &OnSourceClick, cx: &App) -> AnyElement {
    let on_click = on_click.clone();
    let source = row.source;
    div()
        .id(SharedString::from(format!("budgets-fill-{source:?}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(10.0))
        .py(px(10.0))
        .px(px(12.0))
        .border_1()
        .border_color(if selected {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .when(selected, |this| this.bg(color::accent_tint(cx)))
        .text_size(px(13.0))
        .on_click(move |_event, window, cx| on_click(source, window, cx))
        .child(
            div()
                .size(px(14.0))
                .flex_none()
                .rounded_full()
                .border_1()
                .flex()
                .items_center()
                .justify_center()
                .border_color(if selected {
                    color::accent(cx)
                } else {
                    color::muted(cx)
                })
                .when(selected, |this| {
                    this.child(div().size(px(6.0)).rounded_full().bg(color::accent(cx)))
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(row.title),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::muted(cx))
                        .child(row.detail),
                ),
        )
        .child(
            div()
                .flex_none()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(row.total),
        )
        .into_any_element()
}

/// One diff line: the Category, then `before → after` and the signed difference.
fn diff_line(name: String, figures: String, delta: String, cx: &App) -> AnyElement {
    div()
        .flex()
        .gap(px(12.0))
        .py(px(6.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.5))
        .child(div().flex_1().min_w(px(0.0)).child(name))
        .child(div().flex_none().child(figures))
        .child(
            div()
                .w(px(90.0))
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .text_color(color::muted(cx))
                .child(delta),
        )
        .into_any_element()
}

pub fn render(props: FillDialogProps<'_>, cx: &App) -> AnyElement {
    let FillDialogProps {
        month,
        sources,
        preview,
        change_names,
        kept_names,
        on_source_click,
        on_cancel,
        on_confirm,
    } = props;

    let mut fields: Vec<AnyElement> = sources
        .into_iter()
        .map(|row| {
            let selected = row.source == preview.source;
            source_row(row, selected, &on_source_click, cx)
        })
        .collect();

    let mut diff = vec![
        div()
            .pb(px(6.0))
            .border_b(px(1.0))
            .border_color(color::hairline(cx))
            .text_size(px(10.0))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::muted(cx))
            .child(upper(&crate::msg::desktop_budgets_fill_changes()))
            .into_any_element(),
    ];
    for (line, name) in preview.changes.iter().zip(change_names) {
        let before = line.before.as_ref();
        let delta =
            Money(line.after.0.clone() - before.map_or_else(|| 0.into(), |value| value.0.clone()));
        diff.push(diff_line(
            name,
            crate::msg::desktop_budgets_fill_change(
                &before.map_or_else(|| EMPTY_CELL.to_string(), money),
                &money(&line.after),
            ),
            signed_amount(&delta).1,
            cx,
        ));
    }
    for name in kept_names {
        diff.push(diff_line(
            name,
            String::new(),
            crate::msg::desktop_budgets_fill_kept(),
            cx,
        ));
    }
    let muted_line = |text: String| {
        div()
            .pt(px(6.0))
            .text_size(px(12.0))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    if preview.changes.is_empty() {
        diff.push(muted_line(crate::msg::desktop_budgets_fill_no_changes()));
    }
    if preview.unchanged > 0 {
        diff.push(muted_line(crate::msg::desktop_budgets_fill_unchanged(
            i64::try_from(preview.unchanged).unwrap_or(i64::MAX),
        )));
    }
    fields.push(div().flex().flex_col().children(diff).into_any_element());
    fields.push(
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_budgets_fill_note())
            .into_any_element(),
    );

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_budgets_fill_title(&month),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-fill-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "budgets-fill-confirm",
                    crate::msg::desktop_budgets_fill_submit(&month),
                    !preview.changes.is_empty(),
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
