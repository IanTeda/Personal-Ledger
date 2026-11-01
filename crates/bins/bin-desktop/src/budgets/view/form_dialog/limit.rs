//! Edit budget (9e) on the shared `crate::dialog` frame at the handoff's 480px: the Category
//! picker when opened from **+ Budget a category**, Amount per month, Starting, the Applies to and
//! Rollover segmented controls, the before/after summary and the note.
//!
//! The selects take the arrows and `Space` while they have focus, `Enter` commits an open list,
//! else confirms. The Amount takes digits and one decimal point; nothing else takes text.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{LimitField, LimitFormEvent, LimitFormPresenter, LimitFormViewModel};
use lib_budgets::{Rollover, Span};

use super::{Clicks, Form, FormDialog, picker, segmented};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::dialog_fields::{label, text_field};
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select, typed};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};
use crate::theme::color;

use super::super::dialog::DialogRouter;

/// The dialog's width: the handoff's 9e.
const WIDTH: gpui::Pixels = px(480.0);

fn handle(presenter: &mut LimitFormPresenter, event: LimitFormEvent) {
    presenter.handle(event, &mut DialogRouter::default());
}

pub(super) fn key(
    presenter: &mut LimitFormPresenter,
    category: &mut SelectState,
    starting: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let focused = view.focused;
    let list_open = category.is_open() || starting.is_open();
    match key {
        // Leaving a select closes its list, as a click elsewhere would.
        DialogKey::Tab | DialogKey::BackTab => {
            category.cancel();
            starting.cancel();
            let event = if key == DialogKey::Tab {
                LimitFormEvent::NextField
            } else {
                LimitFormEvent::PrevField
            };
            handle(presenter, event);
            return DialogOutcome::Handled;
        }
        DialogKey::Enter if !list_open => return enter(view.frame.can_confirm),
        DialogKey::Left | DialogKey::Right => {
            let event = if key == DialogKey::Right {
                LimitFormEvent::StepForward
            } else {
                LimitFormEvent::StepBack
            };
            handle(presenter, event);
            return DialogOutcome::Handled;
        }
        _ => {}
    }
    match focused {
        LimitField::Category => match key {
            DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
                if let Some(select) = &view.category {
                    let (_, value) = select_key(category, select, key);
                    if let Some(id) = value {
                        handle(presenter, LimitFormEvent::Category(id));
                    }
                }
                DialogOutcome::Handled
            }
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
        LimitField::Starting => match key {
            DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
                let (_, value) = select_key(starting, &view.starting, key);
                if let Some(month) = value {
                    handle(presenter, LimitFormEvent::Starting(month));
                }
                DialogOutcome::Handled
            }
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
        LimitField::Amount => match key {
            DialogKey::Char(ch)
                if !(ch.is_ascii_digit() || (ch == '.' && !view.amount.text.contains('.'))) =>
            {
                DialogOutcome::Handled
            }
            _ => match typed(&view.amount.text, key) {
                Some(text) => {
                    handle(presenter, LimitFormEvent::Amount(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            },
        },
        LimitField::Span | LimitField::Rollover => match key {
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
    }
}

type Parts<'a> = (
    &'a mut LimitFormPresenter,
    &'a mut SelectState,
    &'a mut SelectState,
);

fn form(dialog: &mut FormDialog) -> Option<Parts<'_>> {
    match &mut dialog.view {
        Form::Limit(presenter, category, starting) => Some((presenter, category, starting)),
        _ => None,
    }
}

/// Moves focus to `field`, closing the lists as a click elsewhere does.
fn focus(dialog: &mut FormDialog, field: LimitField) {
    if let Some((presenter, category, starting)) = form(dialog) {
        category.cancel();
        starting.cancel();
        handle(presenter, LimitFormEvent::Focus(field));
    }
}

/// A click on the Category field: focuses it and toggles its list.
fn click_category(dialog: &mut FormDialog) {
    if let Some((presenter, category, starting)) = form(dialog) {
        if presenter.focused() != LimitField::Category {
            category.cancel();
            starting.cancel();
            handle(presenter, LimitFormEvent::Focus(LimitField::Category));
        }
        if let Some(select) = presenter.view_model().category {
            toggle_select(category, &select);
        }
    }
}

fn click_starting(dialog: &mut FormDialog) {
    if let Some((presenter, category, starting)) = form(dialog) {
        if presenter.focused() != LimitField::Starting {
            category.cancel();
            starting.cancel();
            handle(presenter, LimitFormEvent::Focus(LimitField::Starting));
        }
        toggle_select(starting, &presenter.view_model().starting);
    }
}

fn choose_category(dialog: &mut FormDialog, index: usize) {
    if let Some((presenter, category, _)) = form(dialog)
        && let Some(select) = presenter.view_model().category
        && let Some(id) = choose_option(category, &select, index)
    {
        handle(presenter, LimitFormEvent::Category(id));
    }
}

fn choose_starting(dialog: &mut FormDialog, index: usize) {
    if let Some((presenter, _, starting)) = form(dialog)
        && let Some(month) = choose_option(starting, &presenter.view_model().starting, index)
    {
        handle(presenter, LimitFormEvent::Starting(month));
    }
}

fn set_span(dialog: &mut FormDialog, span: Span) {
    if let Some((presenter, category, starting)) = form(dialog) {
        category.cancel();
        starting.cancel();
        handle(presenter, LimitFormEvent::Span(span));
    }
}

fn set_rollover(dialog: &mut FormDialog, rollover: Rollover) {
    if let Some((presenter, category, starting)) = form(dialog) {
        category.cancel();
        starting.cancel();
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
    category: &SelectState,
    starting: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let mut fields = Vec::new();
    if let Some(select) = &view.category {
        fields.push(select_field::render(
            SelectFieldProps {
                id: "budgets-limit-category".into(),
                label: select.label.clone().into(),
                options: &labels(select),
                state: &synced(category, select),
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
    fields.push(select_field::render(
        SelectFieldProps {
            id: "budgets-limit-starting".into(),
            label: view.starting.label.clone().into(),
            options: &labels(&view.starting),
            state: &synced(starting, &view.starting),
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
                if let Form::Limit(presenter, ..) = view {
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
