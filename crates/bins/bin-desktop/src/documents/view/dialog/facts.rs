//! **Edit extracted facts** on the shared `crate::dialog` frame: an Unfiled Document's merchant,
//! date, total and Document Type. The Type select takes the arrows, `Space` and `Enter` while
//! focused and swallows typing; the other three take text.

use gpui::{AnyElement, App, div, prelude::*};
use lib_documents::presenter::{
    FactsField, FactsFormEvent, FactsFormPresenter, FactsFormViewModel,
};
use lib_viper::view::TextFieldView;

use super::document_form::with_error;
use super::{Clicks, DocumentsDialog, View, send};
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select, typed};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

fn handle(presenter: &mut FactsFormPresenter, event: FactsFormEvent) {
    send(presenter, event, FactsFormPresenter::handle);
}

pub(super) fn key(
    presenter: &mut FactsFormPresenter,
    doc_type: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    match (key, view.focused) {
        (DialogKey::Tab | DialogKey::BackTab, _) => {
            doc_type.cancel();
            let event = if key == DialogKey::Tab {
                FactsFormEvent::NextField
            } else {
                FactsFormEvent::PrevField
            };
            handle(presenter, event);
        }
        (
            DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down,
            FactsField::Type,
        ) => {
            if let (_, Some(kind)) = select_key(doc_type, &view.doc_type, key) {
                handle(presenter, FactsFormEvent::Type(kind));
            }
        }
        (DialogKey::Enter | DialogKey::CtrlEnter, _) => return enter(view.frame.can_confirm),
        (DialogKey::Char(_) | DialogKey::Backspace, FactsField::Type) => {}
        (DialogKey::Char(_) | DialogKey::Backspace, field) => {
            let (text, event): (&str, fn(String) -> FactsFormEvent) = match field {
                FactsField::Merchant => (&view.merchant.text, FactsFormEvent::Merchant),
                FactsField::Date => (&view.date.text, FactsFormEvent::Date),
                _ => (&view.total.text, FactsFormEvent::Total),
            };
            if let Some(text) = typed(text, key) {
                handle(presenter, event(text));
            }
        }
        _ => return DialogOutcome::Ignored,
    }
    DialogOutcome::Handled
}

fn form(dialog: &mut DocumentsDialog) -> Option<(&mut FactsFormPresenter, &mut SelectState)> {
    match &mut dialog.view {
        View::Facts(presenter, doc_type) => Some((presenter, doc_type)),
        _ => None,
    }
}

fn click_field(dialog: &mut DocumentsDialog, field: FactsField) {
    if let Some((presenter, doc_type)) = form(dialog) {
        handle(presenter, FactsFormEvent::Focus(field));
        if field == FactsField::Type {
            toggle_select(doc_type, &presenter.view_model().doc_type);
        } else {
            doc_type.cancel();
        }
    }
}

fn choose_type(dialog: &mut DocumentsDialog, index: usize) {
    if let Some((presenter, doc_type)) = form(dialog)
        && let Some(kind) = choose_option(doc_type, &presenter.view_model().doc_type, index)
    {
        handle(presenter, FactsFormEvent::Type(kind));
    }
}

pub(super) fn render(
    view: &FactsFormViewModel,
    doc_type: &SelectState,
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
            state: &synced(doc_type, &view.doc_type),
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
