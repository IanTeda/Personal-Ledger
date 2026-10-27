//! The Remove location Dialog on the shared `crate::dialog` frame: a plain confirm for an empty
//! Location and otherwise the red dialog that quotes what goes with it and asks for the name typed
//! back, which holds every typed key.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_inventory::presenter::{
    RemoveLocationEvent, RemoveLocationPresenter, RemoveLocationViewModel,
};

use super::Clicks;
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::fields::text_field;
use crate::form::viper::{enter, typed};
use crate::inventory::router::DialogRouter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

pub(super) fn key(presenter: &mut RemoveLocationPresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        // The typed-back name is the only field, and swallows typing even when there is none.
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        DialogKey::Char(_) | DialogKey::Backspace => {
            if let Some(text) = view
                .confirm
                .as_ref()
                .and_then(|field| typed(&field.text, key))
            {
                presenter.handle(
                    RemoveLocationEvent::ConfirmText(text),
                    &mut DialogRouter::default(),
                );
            }
            DialogOutcome::Handled
        }
        _ => DialogOutcome::Ignored,
    }
}

pub(super) fn render(view: &RemoveLocationViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut body = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    body.extend(view.confirm.as_ref().map(|field| {
        text_field(
            "remove-location-confirm-input",
            field.label.clone(),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.click(|_| {}),
            cx,
        )
        .into_any_element()
    }));
    body.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-location-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-location-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, destructive, card, cx)
}
