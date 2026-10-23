//! Delete price source (16z·del, 420px) in 16d's red treatment: how many Units the source values,
//! which fall back to Manual entry, and the source's name typed back before the confirm enables.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::presenter::{
    DeletePriceSourceEvent, DeletePriceSourcePresenter, DeletePriceSourceViewModel,
};

use super::{Clicks, enter, refusal, send, typed};
use crate::{
    chrome::dialog_host::{DialogKey, DialogOutcome},
    dialog,
    form::fields::text_field,
};

pub(super) fn key(presenter: &mut DeletePriceSourcePresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => match typed(&view.confirm.text, key) {
            Some(text) => {
                send(
                    presenter,
                    DeletePriceSourceEvent::ConfirmText(text),
                    DeletePriceSourcePresenter::handle,
                );
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

pub(super) fn render(view: &DeletePriceSourceViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    fields.extend(view.fallback.clone().map(|fallback| {
        div()
            .debug_selector(|| "delete-price-source-fallback".to_string())
            .child(dialog::info_panel(fallback, cx))
            .into_any_element()
    }));
    fields.push(
        text_field(
            "delete-price-source-confirm-input",
            view.confirm.label.clone(),
            &view.confirm.text,
            &view.confirm.placeholder,
            view.confirm.focused,
            Rc::new(|_window: &mut Window, _cx: &mut App| {}),
            cx,
        )
        .into_any_element(),
    );
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), true, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-price-source-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-price-source-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    true,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, true, card, cx)
}

pub(super) fn readback(view: &DeletePriceSourceViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("confirm", view.confirm.text.clone()),
        ("warning", view.warning.clone()),
        ("fallback", view.fallback.clone().unwrap_or_default()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
