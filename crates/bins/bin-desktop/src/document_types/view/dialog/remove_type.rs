//! The Remove Document type Dialog on the shared `crate::dialog` frame. A type with Filed files
//! gets the red treatment and a destination dropdown; a type with none, a plain confirm.
//!
//! The Presenter reads every Dialog command and typed key itself; the View draws its view model
//! and turns each click into an event.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::{RemoveTypeEvent, RemoveTypePresenter, RemoveTypeViewModel};

use super::{Clicks, DocumentTypesDialog, View, send};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{chosen_label, drawn, labels};
use crate::{dialog, theme::color};

fn handle(presenter: &mut RemoveTypePresenter, event: RemoveTypeEvent) {
    send(presenter, event, RemoveTypePresenter::handle);
}

/// A click on the destination field opens or closes its list.
fn click_field(dialog: &mut DocumentTypesDialog) {
    if let View::Remove(presenter, _) = &mut dialog.view {
        handle(presenter, RemoveTypeEvent::ToggleDestinations);
    }
}

/// A click on row `index` of the open list.
fn click_option(dialog: &mut DocumentTypesDialog, index: usize) {
    if let View::Remove(presenter, _) = &mut dialog.view
        && let Some(destination) = presenter.view_model().destination
        && let Some(choice) = destination.options.get(index)
    {
        handle(
            presenter,
            RemoveTypeEvent::Destination(choice.value.clone()),
        );
    }
}

pub(super) fn render(
    view: &RemoveTypeViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut body: Vec<AnyElement> = vec![
        div()
            .text_size(px(13.0))
            .child(view.message.clone())
            .into_any_element(),
    ];
    if let Some(destination) = &view.destination {
        body.push(select_field::render(
            SelectFieldProps {
                id: "document-type-destination".into(),
                label: destination.label.clone().into(),
                options: &labels(destination),
                state: &drawn(destination, view.destination_list, scroll),
                focused: true,
                accent: false,
                read_only: None,
                on_field_click: actions.click(click_field),
                on_option_click: actions.option(click_option),
            },
            cx,
        ));
    }
    body.extend(
        view.moved_note
            .clone()
            .map(|note| dialog::info_panel(note, cx).into_any_element()),
    );
    body.extend(view.refusal.clone().map(|text| {
        div()
            .text_size(px(11.5))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::accent_text(cx))
            .child(text)
            .into_any_element()
    }));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-document-type-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-document-type-confirm",
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

/// The destination chosen and what the Dialog says, for a test.
pub(super) fn readback(view: &RemoveTypeViewModel) -> Vec<(&'static str, String)> {
    vec![
        (
            "destination",
            view.destination
                .as_ref()
                .and_then(chosen_label)
                .unwrap_or_default(),
        ),
        ("message", view.message.clone()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
