//! Edit budget (9e) on the shared `crate::dialog` frame at the handoff's 480px: the Category
//! picker when opened from **+ Budget a category**, Amount per month, Starting, the Applies to and
//! Rollover segmented controls, the before/after summary and the note.
//!
//! What each key means is the Presenter's (`lib_budgets::presenter::LimitFormPresenter`); this View
//! draws the form and turns its clicks into Presenter events.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{LimitField, LimitFormEvent, LimitFormPresenter, LimitFormViewModel};
use lib_budgets::{Rollover, Span};

use super::{Clicks, Form, FormDialog, Router, picker, segmented};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::dialog_fields::{label, text_field};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::theme::color;

/// The dialog's width: the handoff's 9e.
const WIDTH: gpui::Pixels = px(480.0);

fn handle(presenter: &mut LimitFormPresenter, event: LimitFormEvent) {
    presenter.handle(event, &mut Router::default());
}

fn form(dialog: &mut FormDialog) -> Option<&mut LimitFormPresenter> {
    match &mut dialog.view {
        Form::Limit(presenter, _) => Some(presenter),
        _ => None,
    }
}

/// Moves focus to `field`, closing an open list as a click elsewhere does.
fn focus(dialog: &mut FormDialog, field: LimitField) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LimitFormEvent::Focus(field));
    }
}

/// A click on the Category field: focuses it and toggles its list.
fn click_category(dialog: &mut FormDialog) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LimitFormEvent::ToggleList(LimitField::Category));
    }
}

fn click_starting(dialog: &mut FormDialog) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LimitFormEvent::ToggleList(LimitField::Starting));
    }
}

fn choose_category(dialog: &mut FormDialog, index: usize) {
    if let Some(presenter) = form(dialog)
        && let Some(select) = presenter.view_model().category
        && let Some(choice) = select.options.get(index)
    {
        handle(presenter, LimitFormEvent::Category(choice.value));
    }
}

fn choose_starting(dialog: &mut FormDialog, index: usize) {
    if let Some(presenter) = form(dialog)
        && let Some(choice) = presenter.view_model().starting.options.get(index)
    {
        handle(presenter, LimitFormEvent::Starting(choice.value));
    }
}

fn set_span(dialog: &mut FormDialog, span: Span) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LimitFormEvent::Span(span));
    }
}

fn set_rollover(dialog: &mut FormDialog, rollover: Rollover) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LimitFormEvent::Rollover(rollover));
    }
}

/// One summary line: what it is on the left, the figures on the right.
fn preview_line(what: String, figures: String, strong: bool, cx: &App) -> AnyElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .text_size(px(12.5))
        .child(div().text_color(color::muted(cx)).child(what))
        .child(
            div()
                .when(strong, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(figures),
        )
        .into_any_element()
}

pub(super) fn render(
    view: &LimitFormViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let mut fields = Vec::new();
    // Only the focused select's list can be open, and only its window is kept.
    let mut unused = 0;
    if let Some(select) = &view.category {
        let scroll = if select.focused {
            &mut *scroll
        } else {
            &mut unused
        };
        fields.push(select_field::render(
            SelectFieldProps {
                id: "budgets-limit-category".into(),
                label: select.label.clone().into(),
                options: &labels(select),
                state: &drawn(select, view.list, scroll),
                focused: select.focused,
                accent: false,
                read_only: None,
                on_field_click: actions.click(click_category),
                on_option_click: actions.option(choose_category),
            },
            cx,
        ));
    }
    if let Some(none) = &view.no_categories {
        fields.push(dialog::info_panel(none.clone(), cx).into_any_element());
    }
    fields.push(
        div()
            .child(text_field(
                "budgets-limit-amount",
                label(view.amount.label.clone()),
                &view.amount.text,
                &view.amount.placeholder,
                view.amount.focused,
                actions.click(|dialog| focus(dialog, LimitField::Amount)),
                cx,
            ))
            .children(view.amount_error.clone().map(|text| error_line(text, cx)))
            .into_any_element(),
    );
    let mut unused = 0;
    let scroll = if view.starting.focused {
        scroll
    } else {
        &mut unused
    };
    fields.push(select_field::render(
        SelectFieldProps {
            id: "budgets-limit-starting".into(),
            label: view.starting.label.clone().into(),
            options: &labels(&view.starting),
            state: &drawn(&view.starting, view.list, scroll),
            focused: view.starting.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.click(click_starting),
            on_option_click: actions.option(choose_starting),
        },
        cx,
    ));
    let on_span = actions.choose(set_span);
    fields.push(segmented(
        "budgets-limit-span",
        view.span.label.clone(),
        view.span.focused,
        view.span
            .options
            .iter()
            .map(|choice| {
                (
                    choice.label.clone(),
                    view.span.chosen == Some(choice.value),
                    picker(on_span.clone(), choice.value),
                )
            })
            .collect(),
        cx,
    ));
    let on_rollover = actions.choose(set_rollover);
    fields.push(segmented(
        "budgets-limit-rollover",
        view.rollover.label.clone(),
        view.rollover.focused,
        view.rollover
            .options
            .iter()
            .map(|choice| {
                (
                    choice.label.clone(),
                    view.rollover.chosen == Some(choice.value),
                    picker(on_rollover.clone(), choice.value),
                )
            })
            .collect(),
        cx,
    ));
    if !view.preview.is_empty() {
        fields.push(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .p(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .children(view.preview.iter().map(|line| {
                    preview_line(line.what.clone(), line.figures.clone(), line.strong, cx)
                }))
                .into_any_element(),
        );
    }
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

    let mut buttons = Vec::new();
    if let Some(stop) = &view.stop {
        let on_stop = actions.act(|dialog, cx| {
            dialog.run(cx, |view, router| {
                if let Form::Limit(presenter, _) = view {
                    presenter.handle(LimitFormEvent::StopBudgeting, router);
                }
            });
        });
        buttons.push(
            div()
                .id("budgets-limit-stop")
                .debug_selector(|| "budgets-limit-stop".to_string())
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .mr_auto()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .on_click(move |_event, window, cx| on_stop(window, cx))
                .child(stop.clone())
                .into_any_element(),
        );
    }
    buttons.push(
        dialog::cancel_button("budgets-limit-cancel", actions.cancel.clone(), cx)
            .into_any_element(),
    );
    buttons.push(
        dialog::confirm_button(
            "budgets-limit-confirm",
            view.frame.submit.clone(),
            view.frame.can_confirm,
            false,
            actions.confirm.clone(),
            cx,
        )
        .into_any_element(),
    );

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(buttons, cx));
    dialog::overlay(WIDTH, false, card, cx)
}
