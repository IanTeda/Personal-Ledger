//! Fill from… (9f) on the shared `crate::dialog` frame at the handoff's 560px: the two sources as
//! radio rows, each with the Total budgeted it would give the month, then the chosen source's diff
//! against the plan. "The plan" is the baseline rather than a source (#386).

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_budgets::FillSource;
use lib_budgets::presenter::{ChangeRow, FillEvent, FillPresenter, FillViewModel, SourceRow};
use lib_locale::format::upper;

use super::{Clicks, Form, FormDialog};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::viper::enter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};
use crate::theme::color;

use super::super::dialog::DialogRouter;

/// The dialog's width: the handoff's 9f.
const WIDTH: gpui::Pixels = px(560.0);

/// `j`/`k` pick the source; `enter` fills.
pub(super) fn key(presenter: &mut FillPresenter, key: DialogKey) -> DialogOutcome {
    let event = match key {
        DialogKey::Down | DialogKey::Char('j') => FillEvent::Down,
        DialogKey::Up | DialogKey::Char('k') => FillEvent::Up,
        DialogKey::Enter => return enter(presenter.view_model().frame.can_confirm),
        _ => return DialogOutcome::Ignored,
    };
    presenter.handle(event, &mut DialogRouter::default());
    DialogOutcome::Handled
}

fn choose(dialog: &mut FormDialog, source: FillSource) {
    if let Form::Fill(presenter) = &mut dialog.view {
        presenter.handle(FillEvent::Source(source), &mut DialogRouter::default());
    }
}

/// A bordered radio row; the chosen one gets the accent border and tint.
fn source_row(row: &SourceRow, on_click: dialog::OnClick, cx: &App) -> AnyElement {
    let selected = row.chosen;
    let source = row.source;
    div()
        .id(SharedString::from(format!("budgets-fill-{source:?}")))
        .debug_selector(move || format!("budgets-fill-{source:?}"))
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
        .on_click(move |_event, window, cx| on_click(window, cx))
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
                        .child(row.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::muted(cx))
                        .child(row.detail.clone()),
                ),
        )
        .child(
            div()
                .flex_none()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(row.total.clone()),
        )
        .into_any_element()
}

/// One diff line: the Category, then `before → after` and the signed difference.
fn diff_line(row: &ChangeRow, cx: &App) -> AnyElement {
    div()
        .flex()
        .gap(px(12.0))
        .py(px(6.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.5))
        .child(div().flex_1().min_w(px(0.0)).child(row.name.clone()))
        .child(div().flex_none().child(row.figures.clone()))
        .child(
            div()
                .w(px(90.0))
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .text_color(color::muted(cx))
                .child(row.delta.clone()),
        )
        .into_any_element()
}

pub(super) fn render(view: &FillViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields: Vec<AnyElement> = view
        .sources
        .iter()
        .map(|row| {
            let source = row.source;
            source_row(row, actions.click(move |dialog| choose(dialog, source)), cx)
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
            .child(upper(&view.changes_title))
            .into_any_element(),
    ];
    diff.extend(view.changes.iter().map(|row| diff_line(row, cx)));
    diff.extend(view.kept.iter().map(|row| diff_line(row, cx)));
    let muted_line = |text: String| {
        div()
            .pt(px(6.0))
            .text_size(px(12.0))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    diff.extend(view.no_changes.clone().map(muted_line));
    diff.extend(view.unchanged.clone().map(muted_line));
    fields.push(div().flex().flex_col().children(diff).into_any_element());
    fields.push(
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(view.note.clone())
            .into_any_element(),
    );
    if let Some(error) = &view.error {
        fields.push(error_line(error.clone(), cx));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-fill-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "budgets-fill-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
