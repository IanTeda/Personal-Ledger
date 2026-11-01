//! Stop budgeting (9g) on the shared `crate::dialog` frame at the handoff's 420px. A Stop deletes
//! nothing (a later Onward amount resumes the Category), so there's no typed confirmation and the
//! confirm button keeps the neutral dark fill rather than the destructive one.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{StopEvent, StopPresenter, StopViewModel};

use super::{Clicks, Form, FormDialog};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

use super::super::dialog::DialogRouter;

/// The dialog's width: the handoff's 9g.
const WIDTH: gpui::Pixels = px(420.0);

/// `↑`/`↓` (and `j`/`k`) pick the month and `Space` opens or commits its list, as `Enter` does
/// while it is open (otherwise `Enter` confirms). Nothing takes text.
pub(super) fn key(
    presenter: &mut StopPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let key = match key {
        DialogKey::Char('k') => DialogKey::Up,
        DialogKey::Char('j') => DialogKey::Down,
        other => other,
    };
    match key {
        DialogKey::Enter if !select.is_open() => enter(view.frame.can_confirm),
        DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
            let (_, value) = select_key(select, &view.from, key);
            if let Some(month) = value {
                presenter.handle(StopEvent::From(month), &mut DialogRouter::default());
            }
            DialogOutcome::Handled
        }
        _ => DialogOutcome::Ignored,
    }
}

fn form(dialog: &mut FormDialog) -> Option<(&mut StopPresenter, &mut SelectState)> {
    match &mut dialog.view {
        Form::Stop(presenter, select) => Some((presenter, select)),
        _ => None,
    }
}

fn click_from(dialog: &mut FormDialog) {
    if let Some((presenter, select)) = form(dialog) {
        toggle_select(select, &presenter.view_model().from);
    }
}

fn click_option(dialog: &mut FormDialog, index: usize) {
    if let Some((presenter, select)) = form(dialog)
        && let Some(month) = choose_option(select, &presenter.view_model().from, index)
    {
        presenter.handle(StopEvent::From(month), &mut DialogRouter::default());
    }
}

pub(super) fn render(
    view: &StopViewModel,
    select: &SelectState,
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
                state: &synced(select, &view.from),
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
