//! The Delete payee Dialog (`docs/ux/desktop-mockups/20-payees/README.md`'s 6d) on the shared
//! `crate::dialog` frame at the handoff's 440px, and the Deactivate and Reactivate Dialogs it turns
//! into (#283). The typed-back name holds every typed key; reactivating has no field, and drops the
//! red frame.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_payees::presenter::{DeletePayeeEvent, DeletePayeePresenter, DeletePayeeViewModel};

use super::rules_field::error_line;
use super::{Clicks, send};
use crate::form::viper::{enter, typed};
use crate::{
    dialog,
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

/// The Dialog's width: the handoff's 6d.
const WIDTH: gpui::Pixels = px(440.0);

pub(super) fn key(presenter: &mut DeletePayeePresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => match view
            .confirm
            .as_ref()
            .and_then(|confirm| typed(&confirm.text, key))
        {
            Some(text) => {
                send(
                    presenter,
                    DeletePayeeEvent::ConfirmText(text),
                    DeletePayeePresenter::handle,
                );
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

pub(super) fn render(view: &DeletePayeeViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut fields = vec![warning(view), callout(view, cx)];
    fields.extend(
        view.confirm
            .as_ref()
            .map(|confirm| confirm_field(&confirm.label, &confirm.text, &confirm.placeholder, cx)),
    );
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-payee-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-payee-confirm",
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
    dialog::overlay(WIDTH, destructive, card, cx)
}

fn warning(view: &DeletePayeeViewModel) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .text_size(px(13.0))
        .children(view.warning.iter().map(|segment| {
            let text = div().child(segment.text.clone());
            match segment.tag.as_deref() {
                Some("strong") => text.font_weight(gpui::FontWeight::EXTRA_BOLD),
                _ => text,
            }
        }))
        .into_any_element()
}

/// The handoff's red-bordered callout on a destructive action; reactivation's is the neutral
/// `info_panel`.
fn callout(view: &DeletePayeeViewModel, cx: &App) -> AnyElement {
    let text = view.callout.clone();
    if !view.frame.destructive {
        return dialog::info_panel(text, cx).into_any_element();
    }
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::negative(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// The typed-back name, its placeholder the name itself.
fn confirm_field(label: &str, text: &str, placeholder: &str, cx: &App) -> AnyElement {
    let (shown, text_color) = if text.is_empty() {
        (placeholder.to_string(), color::muted(cx))
    } else {
        (text.to_string(), color::foreground(cx))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label.to_string()),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(13.0))
                .text_color(text_color)
                .child(SharedString::from(shown)),
        )
        .into_any_element()
}

/// The typed-back name and the action, for a test.
pub(super) fn readback(view: &DeletePayeeViewModel) -> Vec<(&'static str, String)> {
    vec![
        (
            "confirm",
            view.confirm
                .as_ref()
                .map(|confirm| confirm.text.clone())
                .unwrap_or_default(),
        ),
        ("action", format!("{:?}", view.action)),
        (
            "warning",
            view.warning
                .iter()
                .map(|segment| segment.text.as_str())
                .collect(),
        ),
    ]
}
