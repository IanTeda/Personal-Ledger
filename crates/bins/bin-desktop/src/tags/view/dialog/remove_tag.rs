//! The Remove tag Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7d) on the shared
//! `crate::dialog` frame at the handoff's 400px. Removing always deletes the Tag (#353) and keeps
//! the neutral (not red) styling: nothing but a label is lost. A used Tag sits behind the
//! typed-back name, which holds every typed key; an unused one has no field.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_tags::presenter::{RemoveTagEvent, RemoveTagPresenter, RemoveTagViewModel};

use super::Clicks;
use crate::form::colour_field::error_line;
use crate::form::viper::{enter, typed};
use crate::{
    dialog,
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

/// The Dialog's width: the handoff's 7d.
const WIDTH: gpui::Pixels = px(400.0);

pub(super) fn key(presenter: &mut RemoveTagPresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        // The typed-back name is the only field.
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => match view
            .confirm
            .as_ref()
            .and_then(|field| typed(&field.text, key))
        {
            Some(text) => {
                presenter.handle(
                    RemoveTagEvent::ConfirmText(text),
                    &mut crate::tags::router::DialogRouter::default(),
                );
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

pub(super) fn render(view: &RemoveTagViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![warning(view)];
    fields.extend(
        view.confirm
            .as_ref()
            .map(|field| confirm_field(&field.label, &field.text, &field.placeholder, cx)),
    );
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-tag-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-tag-confirm",
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

fn warning(view: &RemoveTagViewModel) -> AnyElement {
    div()
        .text_size(px(13.0))
        .child(dialog::rich_text(view.warning.clone(), None))
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

/// The typed-back name, for a test: empty when the Tag is unused and asks for none.
pub(super) fn readback(view: &RemoveTagViewModel) -> Vec<(&'static str, String)> {
    vec![
        (
            "confirm",
            view.confirm
                .as_ref()
                .map(|field| field.text.clone())
                .unwrap_or_default(),
        ),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
