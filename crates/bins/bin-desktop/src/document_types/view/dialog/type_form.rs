//! The Add and Edit Document type Dialogs (#478's stacked-form variant A) on the shared
//! `crate::dialog` frame. They share one View: Name, Tracks date, Remind and Financial year stacked
//! with a hint under each.
//!
//! The Presenter reads every Dialog command and typed key itself (`Left`/`Right` step a control,
//! `Space` toggles the Financial year chip); the View draws its view model and turns each click
//! into an event.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_documents::presenter::{
    DocumentTypeField, NAME_MAX, TypeFormEvent, TypeFormPresenter, TypeFormViewModel,
};
use lib_viper::view::SelectView;

use super::{Clicks, DocumentTypesDialog, View, send};
use crate::form::fields::{field_label, segmented_labelled, text_field};
use crate::form::viper::chosen_label;
use crate::{dialog, theme::color};

fn handle(presenter: &mut TypeFormPresenter, event: TypeFormEvent) {
    send(presenter, event, TypeFormPresenter::handle);
}

fn form(dialog: &mut DocumentTypesDialog) -> Option<&mut TypeFormPresenter> {
    match &mut dialog.view {
        View::Form(presenter) => Some(presenter),
        View::Remove(..) | View::Notice(_) => None,
    }
}

/// A click on the form sends `event`.
fn click(dialog: &mut DocumentTypesDialog, event: TypeFormEvent) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, event);
    }
}

pub(super) fn render(view: &TypeFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let (prefix, name_id) = if view.editing {
        ("edit-document-type", "document-type-name")
    } else {
        ("add-document-type", "document-type-name")
    };
    let mut fields: Vec<AnyElement> = Vec::new();
    if let Some(usage) = &view.usage {
        fields.push(dialog::info_panel(usage.clone(), cx).into_any_element());
    }
    fields.push(
        div()
            .child(text_field(
                name_id,
                view.name.label.clone(),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                actions
                    .click(|dialog| click(dialog, TypeFormEvent::Focus(DocumentTypeField::Name))),
                cx,
            ))
            .child(match &view.name_error {
                Some(error) => div()
                    .mt(px(4.0))
                    .text_size(px(11.5))
                    .text_color(color::accent_text(cx))
                    .child(error.clone())
                    .into_any_element(),
                None => hint(view.name_hint.clone(), cx).into_any_element(),
            })
            .into_any_element(),
    );
    fields.push(
        div()
            .child(field_label(view.tracks_date.label.clone()))
            .child(segmented_labelled(
                "document-type-tracks",
                options(&view.tracks_date),
                view.tracks_date.chosen.flatten(),
                actions.choose(|dialog, option| click(dialog, TypeFormEvent::TracksDate(option))),
                None,
                cx,
            ))
            .child(hint(view.tracks_date_hint.clone(), cx))
            .into_any_element(),
    );
    fields.push(
        div()
            .when(!view.remind_enabled, |this| this.opacity(0.45))
            .child(field_label(view.remind.label.clone()))
            .child(segmented_labelled(
                "document-type-remind",
                options(&view.remind),
                view.remind.chosen.flatten(),
                actions.choose(|dialog, option| click(dialog, TypeFormEvent::Remind(option))),
                None,
                cx,
            ))
            .child(hint(view.remind_hint.clone(), cx))
            .into_any_element(),
    );
    fields.push(
        div()
            .child(field_label(view.financial_year_label.clone()))
            .child(financial_year_chip(
                view,
                actions.click(|dialog| click(dialog, TypeFormEvent::ToggleFinancialYear)),
                cx,
            ))
            .child(hint(view.financial_year_hint.clone(), cx))
            .into_any_element(),
    );

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
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

/// `select`'s options as a segmented control takes them.
fn options<T: Copy>(select: &SelectView<T>) -> Vec<(T, SharedString)> {
    select
        .options
        .iter()
        .map(|choice| (choice.value, SharedString::from(choice.label.clone())))
        .collect()
}

fn hint(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
}

/// The Financial year toggle chip: filled while on, accent-bordered while focused.
fn financial_year_chip(
    view: &TypeFormViewModel,
    on_click: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    let (on, focused) = (view.financial_year, view.financial_year_focused);
    div().flex().child(
        div()
            .id("document-type-financial-year")
            .debug_selector(|| "document-type-financial-year".to_string())
            .cursor_pointer()
            .py(px(7.0))
            .px(px(12.0))
            .border_1()
            .border_color(if focused {
                color::accent(cx)
            } else {
                color::divider(cx)
            })
            .text_size(px(13.0))
            .when(on, |this| {
                this.bg(color::accent(cx)).text_color(color::background(cx))
            })
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(if on {
                view.yes.clone()
            } else {
                view.no.clone()
            }),
    )
}

/// The form's fields, for a test.
pub(super) fn readback(view: &TypeFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("field", format!("{:?}", view.focused)),
        ("name", view.name.text.clone()),
        (
            "tracks_date",
            chosen_label(&view.tracks_date).unwrap_or_default(),
        ),
        ("remind", chosen_label(&view.remind).unwrap_or_default()),
        ("financial_year", view.financial_year.to_string()),
        ("error", view.name_error.clone().unwrap_or_default()),
        ("name_max", NAME_MAX.to_string()),
    ]
}
