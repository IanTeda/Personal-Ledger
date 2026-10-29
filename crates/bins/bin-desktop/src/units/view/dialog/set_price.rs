//! The Set price Dialog: Week, with the Monday it snaps to under it, then Close in the quote Unit.
//! `Tab` moves between them, and `Enter` confirms once both read.

use gpui::{AnyElement, App, div, prelude::*};
use lib_units::presenter::{SetPriceEvent, SetPriceField, SetPricePresenter, SetPriceViewModel};
use lib_viper::view::TextFieldView;

use super::{Clicks, UnitsDialog, View, enter, help, refusal, send, typed};
use crate::{
    dialog,
    form::dialog_fields::{label, text_field},
    shell::view::dialog_host::{DialogKey, DialogOutcome},
};

fn handle(presenter: &mut SetPricePresenter, event: SetPriceEvent) {
    send(presenter, event, SetPricePresenter::handle);
}

pub(super) fn key(presenter: &mut SetPricePresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Tab => {
            handle(presenter, SetPriceEvent::NextField);
            DialogOutcome::Handled
        }
        DialogKey::BackTab => {
            handle(presenter, SetPriceEvent::PrevField);
            DialogOutcome::Handled
        }
        _ if view.week.focused => match typed(&view.week.text, key) {
            Some(text) => {
                handle(presenter, SetPriceEvent::Week(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        _ => match typed(&view.close.text, key) {
            Some(text) => {
                handle(presenter, SetPriceEvent::Close(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

fn click_text(dialog: &mut UnitsDialog, field: SetPriceField) {
    if let View::SetPrice(presenter) = &mut dialog.view {
        handle(presenter, SetPriceEvent::Focus(field));
    }
}

pub(super) fn render(view: &SetPriceViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let text = |id: &'static str, field: &TextFieldView, which: SetPriceField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            UnitsDialog::on_click(actions, move |dialog| click_text(dialog, which)),
            cx,
        )
    };
    let mut fields = vec![
        div()
            .child(text("set-price-week", &view.week, SetPriceField::Week))
            .child(
                div()
                    .debug_selector(|| "set-price-week-help".to_string())
                    .child(help(view.week_help.clone(), cx)),
            )
            .into_any_element(),
        div()
            .child(text("set-price-close", &view.close, SetPriceField::Close))
            .child(help(view.close_help.clone(), cx))
            .into_any_element(),
    ];
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("set-price-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "set-price-confirm",
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
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

pub(super) fn readback(view: &SetPriceViewModel) -> Vec<(&'static str, String)> {
    let field = if view.week.focused { "Week" } else { "Close" };
    vec![
        ("week", view.week.text.clone()),
        ("week_help", view.week_help.clone()),
        ("close", view.close.text.clone()),
        ("close_help", view.close_help.clone()),
        ("field", field.to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
