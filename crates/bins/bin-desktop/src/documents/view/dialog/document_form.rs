//! **Add document** and **Edit document** on the shared `crate::dialog` frame: one View, Edit
//! without the Path and the note. The Key date's date and reminder show once a kind is chosen.
//!
//! The two selects take the arrows, `Space` and `Enter` while focused; `Space` flips the reminder;
//! the text fields take typing, and a select or the reminder swallows it.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::{
    CheckboxView, DocumentField, DocumentFormEvent, DocumentFormPresenter, DocumentFormViewModel,
};
use lib_viper::view::TextFieldView;

use super::{Clicks, DocumentsDialog, View, send};
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select, typed};
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};
use crate::theme::color;

fn handle(presenter: &mut DocumentFormPresenter, event: DocumentFormEvent) {
    send(presenter, event, DocumentFormPresenter::handle);
}

pub(super) fn key(
    presenter: &mut DocumentFormPresenter,
    doc_type: &mut SelectState,
    key_kind: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let focused = view.focused;
    if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
        // Leaving a select closes its list, as a click elsewhere would.
        doc_type.cancel();
        key_kind.cancel();
        let event = if key == DialogKey::Tab {
            DocumentFormEvent::NextField
        } else {
            DocumentFormEvent::PrevField
        };
        handle(presenter, event);
        return DialogOutcome::Handled;
    }
    let select_keys = matches!(
        key,
        DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down
    );
    match focused {
        DocumentField::Type if select_keys => {
            if let (_, Some(kind)) = select_key(doc_type, &view.doc_type, key) {
                handle(presenter, DocumentFormEvent::Type(kind));
            }
            DialogOutcome::Handled
        }
        DocumentField::KeyKind if select_keys => {
            if let (_, Some(kind)) = select_key(key_kind, &view.key_kind, key) {
                handle(presenter, DocumentFormEvent::KeyKind(kind));
            }
            DialogOutcome::Handled
        }
        _ if matches!(key, DialogKey::Enter | DialogKey::CtrlEnter) => {
            enter(view.frame.can_confirm)
        }
        DocumentField::Reminder if key == DialogKey::Char(' ') => {
            handle(presenter, DocumentFormEvent::ToggleReminder);
            DialogOutcome::Handled
        }
        DocumentField::Path
        | DocumentField::Title
        | DocumentField::Date
        | DocumentField::KeyDate => {
            let (field, event): (Option<&TextFieldView>, fn(String) -> DocumentFormEvent) =
                match focused {
                    DocumentField::Path => (view.path.as_ref(), DocumentFormEvent::Path),
                    DocumentField::Title => (Some(&view.title), DocumentFormEvent::Title),
                    DocumentField::Date => (Some(&view.date), DocumentFormEvent::Date),
                    _ => (view.key_date.as_ref(), DocumentFormEvent::KeyDate),
                };
            match field.and_then(|field| typed(&field.text, key)) {
                Some(text) => {
                    handle(presenter, event(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            }
        }
        // A select or the reminder swallows typing rather than letting it reach the Shell.
        _ if matches!(key, DialogKey::Char(_) | DialogKey::Backspace) => DialogOutcome::Handled,
        _ => DialogOutcome::Ignored,
    }
}

fn form(
    dialog: &mut DocumentsDialog,
) -> Option<(
    &mut DocumentFormPresenter,
    &mut SelectState,
    &mut SelectState,
)> {
    match &mut dialog.view {
        View::Form(presenter, doc_type, key_kind) => Some((presenter, doc_type, key_kind)),
        _ => None,
    }
}

/// A click on a text field or the reminder focuses it; the reminder's also flips it.
fn click_field(dialog: &mut DocumentsDialog, field: DocumentField) {
    if let Some((presenter, doc_type, key_kind)) = form(dialog) {
        doc_type.cancel();
        key_kind.cancel();
        handle(presenter, DocumentFormEvent::Focus(field));
        if field == DocumentField::Reminder {
            handle(presenter, DocumentFormEvent::ToggleReminder);
        }
    }
}

/// A click on a select's field focuses it and toggles its list, closing the other's.
fn click_select(dialog: &mut DocumentsDialog, field: DocumentField) {
    if let Some((presenter, doc_type, key_kind)) = form(dialog) {
        handle(presenter, DocumentFormEvent::Focus(field));
        let view = presenter.view_model();
        if field == DocumentField::Type {
            key_kind.cancel();
            toggle_select(doc_type, &view.doc_type);
        } else {
            doc_type.cancel();
            toggle_select(key_kind, &view.key_kind);
        }
    }
}

fn choose_type(dialog: &mut DocumentsDialog, index: usize) {
    if let Some((presenter, doc_type, _)) = form(dialog) {
        let view = presenter.view_model();
        if let Some(kind) = choose_option(doc_type, &view.doc_type, index) {
            handle(presenter, DocumentFormEvent::Type(kind));
        }
    }
}

fn choose_key_kind(dialog: &mut DocumentsDialog, index: usize) {
    if let Some((presenter, _, key_kind)) = form(dialog) {
        let view = presenter.view_model();
        if let Some(kind) = choose_option(key_kind, &view.key_kind, index) {
            handle(presenter, DocumentFormEvent::KeyKind(kind));
        }
    }
}

pub(super) fn render(
    view: &DocumentFormViewModel,
    doc_type: &SelectState,
    key_kind: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let text = |id: &'static str, field: &TextFieldView, which: DocumentField| {
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
    let select = |id: &'static str,
                  field: DocumentField,
                  options: Vec<String>,
                  state: SelectState,
                  focused: bool,
                  label: String,
                  on_option_click: select_field::OnOptionClick| {
        select_field::render(
            SelectFieldProps {
                id: id.into(),
                label: label.into(),
                options: &options,
                state: &state,
                focused,
                accent: false,
                read_only: None,
                on_field_click: actions.click(move |dialog| click_select(dialog, field)),
                on_option_click,
            },
            cx,
        )
    };

    let mut fields: Vec<AnyElement> = Vec::new();
    if let Some(path) = &view.path {
        fields.push(with_error(
            text("document-path", path, DocumentField::Path),
            view.path_error.clone(),
            cx,
        ));
    }
    fields.push(text("document-title", &view.title, DocumentField::Title));
    fields.push(two_up([
        select(
            "document-type",
            DocumentField::Type,
            labels(&view.doc_type),
            synced(doc_type, &view.doc_type),
            view.doc_type.focused,
            view.doc_type.label.clone(),
            actions.option(choose_type),
        ),
        with_error(
            text("document-date", &view.date, DocumentField::Date),
            view.date_error.clone(),
            cx,
        ),
    ]));
    fields.push(select(
        "document-key-kind",
        DocumentField::KeyKind,
        labels(&view.key_kind),
        synced(key_kind, &view.key_kind),
        view.key_kind.focused,
        view.key_kind.label.clone(),
        actions.option(choose_key_kind),
    ));
    if let Some(key_date) = &view.key_date {
        fields.push(with_error(
            text("document-key-date", key_date, DocumentField::KeyDate),
            view.key_date_error.clone(),
            cx,
        ));
    }
    if let Some(reminder) = &view.reminder {
        fields.push(checkbox(
            reminder,
            actions.click(|dialog| click_field(dialog, DocumentField::Reminder)),
            cx,
        ));
    }
    if let Some(note) = &view.note {
        fields.push(dialog::info_panel(note.clone(), cx).into_any_element());
    }

    let id = if view.editing {
        "edit-document"
    } else {
        "add-document"
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(format!("{id}-cancel"), actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    format!("{id}-confirm"),
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

/// A field with its problem, if any, under it.
pub(super) fn with_error(field: AnyElement, problem: Option<String>, cx: &App) -> AnyElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .child(field)
        .children(problem.map(|text| error_line(text, cx)))
        .into_any_element()
}

pub(super) fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}

/// The Key Date reminder checkbox: a 14px box, filled while checked.
fn checkbox(view: &CheckboxView, on_click: dialog::OnClick, cx: &App) -> AnyElement {
    div()
        .id("document-reminder")
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .text_size(px(13.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(
            div()
                .size(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(if view.focused {
                    color::accent(cx)
                } else {
                    color::foreground(cx)
                })
                .when(view.checked, |this| {
                    this.bg(color::foreground(cx))
                        .text_color(color::background(cx))
                        .text_size(px(10.0))
                        .child("\u{2713}")
                }),
        )
        .child(view.label.clone())
        .into_any_element()
}
