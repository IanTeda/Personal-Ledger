//! Stop budgeting (9g) on the shared `crate::dialog` frame at the handoff's 420px. A Stop deletes
//! nothing (a later Onward amount resumes the Category), so there's no typed confirmation and the
//! confirm button keeps the neutral dark fill rather than the destructive one.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{StopEvent, StopPresenter, StopViewModel};

use super::{Clicks, Form, FormDialog, Router};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};

/// The dialog's width: the handoff's 9g.
const WIDTH: gpui::Pixels = px(420.0);

/// What each key means is the Presenter's (`lib_budgets::presenter::StopPresenter`); this View
/// draws the dialog and turns its clicks into Presenter events.
fn form(dialog: &mut FormDialog) -> Option<&mut StopPresenter> {
    match &mut dialog.view {
        Form::Stop(presenter, _) => Some(presenter),
        _ => None,
    }
}

fn click_from(dialog: &mut FormDialog) {
    if let Some(presenter) = form(dialog) {
        presenter.handle(StopEvent::ToggleList, &mut Router::default());
    }
}

fn click_option(dialog: &mut FormDialog, index: usize) {
    if let Some(presenter) = form(dialog)
        && let Some(choice) = presenter.view_model().from.options.get(index)
    {
        presenter.handle(StopEvent::From(choice.value), &mut Router::default());
    }
}

pub(super) fn render(
    view: &StopViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let options = labels(&view.from);
    let mut fields = vec![
        select_field::render(
            SelectFieldProps {
                id: "budgets-stop-from".into(),
                label: view.from.label.clone().into(),
                options: &options,
                state: &drawn(&view.from, view.from_list, scroll),
                focused: true,
                accent: false,
                read_only: None,
                on_field_click: actions.click(click_from),
                on_option_click: actions.option(click_option),
            },
            cx,
        ),
        div()
            .text_size(px(13.0))
            .child(view.body.clone())
            .into_any_element(),
    ];
    if let Some(bills) = &view.bills {
        fields.push(
            div()
                .text_size(px(13.0))
                .child(bills.clone())
                .into_any_element(),
        );
    }
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
                dialog::cancel_button("budgets-stop-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "budgets-stop-confirm",
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
