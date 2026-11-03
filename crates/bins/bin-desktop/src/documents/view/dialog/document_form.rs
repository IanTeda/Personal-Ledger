//! **Add document** and **Edit document** on the shared `crate::dialog` frame: one View, Edit
//! without the Path and the note. The Key date's date and reminder show once a kind is chosen.
//!
//! What each key means is the Presenter's; this View draws the form and turns its clicks into
//! Presenter events.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::{
    CheckboxView, DocumentField, DocumentFormEvent, DocumentFormViewModel,
};
use lib_viper::view::{SelectList, TextFieldView};

use super::{Clicks, DocumentsDialog, View};
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::theme::color;

/// A click on a text field or the reminder focuses it; the reminder's also flips it.
fn click_field(dialog: &mut DocumentsDialog, field: DocumentField) {
    dialog.apply(|view, router| {
        if let View::Form(presenter, ..) = view {
            presenter.handle(DocumentFormEvent::Focus(field), router);
            if field == DocumentField::Reminder {
                presenter.handle(DocumentFormEvent::ToggleReminder, router);
            }
        }
    });
}

/// A click on a select's field focuses it and toggles its list, closing the other's.
fn click_select(dialog: &mut DocumentsDialog, field: DocumentField) {
    dialog.apply(|view, router| {
        if let View::Form(presenter, ..) = view {
            presenter.handle(DocumentFormEvent::ToggleList(field), router);
        }
    });
}

fn choose_type(dialog: &mut DocumentsDialog, index: usize) {
    dialog.apply(|view, router| {
        if let View::Form(presenter, ..) = view
            && let Some(choice) = presenter.view_model().doc_type.options.get(index)
        {
            presenter.handle(DocumentFormEvent::Type(choice.value), router);
        }
    });
}

fn choose_key_kind(dialog: &mut DocumentsDialog, index: usize) {
    dialog.apply(|view, router| {
        if let View::Form(presenter, ..) = view
            && let Some(choice) = presenter.view_model().key_kind.options.get(index)
        {
            presenter.handle(DocumentFormEvent::KeyKind(choice.value), router);
        }
    });
}

pub(super) fn render(
    view: &DocumentFormViewModel,
    doc_type_scroll: &mut usize,
    key_kind_scroll: &mut usize,
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
                  state: crate::form::select::SelectState,
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
            drawn(
                &view.doc_type,
                if view.doc_type.focused {
                    view.list
                } else {
                    SelectList::default()
                },
                doc_type_scroll,
            ),
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
        drawn(
            &view.key_kind,
            if view.key_kind.focused {
                view.list
            } else {
                SelectList::default()
            },
            key_kind_scroll,
        ),
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
