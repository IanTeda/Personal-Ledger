//! **Edit extracted facts** on the shared `crate::dialog` frame: an Unfiled Document's merchant,
//! date, total and Document Type. What each key means is the
//! Presenter's; this View draws the form and turns its clicks into Presenter events.

use gpui::{AnyElement, App, div, prelude::*};
use lib_documents::presenter::{FactsField, FactsFormEvent, FactsFormViewModel};
use lib_viper::view::TextFieldView;

use super::document_form::with_error;
use super::{Clicks, DocumentsDialog, View};
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};

fn click_field(dialog: &mut DocumentsDialog, field: FactsField) {
    dialog.apply(|view, router| {
        if let View::Facts(presenter, _) = view {
            let event = if field == FactsField::Type {
                FactsFormEvent::ToggleTypeList
            } else {
                FactsFormEvent::Focus(field)
            };
            presenter.handle(event, router);
        }
    });
}

fn choose_type(dialog: &mut DocumentsDialog, index: usize) {
    dialog.apply(|view, router| {
        if let View::Facts(presenter, _) = view
            && let Some(choice) = presenter.view_model().doc_type.options.get(index)
        {
            presenter.handle(FactsFormEvent::Type(choice.value), router);
        }
    });
}

pub(super) fn render(
    view: &FactsFormViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let text = |id: &'static str, field: &TextFieldView, which: FactsField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.click(move |dialog| click_field(dialog, which)),
            cx,
        )
    };
    let type_field = select_field::render(
        SelectFieldProps {
            id: "facts-type".into(),
            label: view.doc_type.label.clone().into(),
            options: &labels(&view.doc_type),
            state: &drawn(&view.doc_type, view.doc_type_list, scroll),
            focused: view.doc_type.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.click(|dialog| click_field(dialog, FactsField::Type)),
            on_option_click: actions.option(choose_type),
        },
        cx,
    );
    let fields = vec![
        text("facts-merchant", &view.merchant, FactsField::Merchant),
        two_up([
            with_error(
                text("facts-date", &view.date, FactsField::Date),
                view.date_error.clone(),
                cx,
            ),
            with_error(
                text("facts-total", &view.total, FactsField::Total),
                view.total_error.clone(),
                cx,
            ),
        ]),
        type_field,
        dialog::info_panel(view.note.clone(), cx).into_any_element(),
    ];
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("facts-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "facts-confirm",
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
