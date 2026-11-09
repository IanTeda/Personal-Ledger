//! **Add document** and **Edit document** on the shared `crate::dialog` frame: one View, Edit
//! without the Path and the note. The Key date's date and reminder show once a kind is chosen.
//!
//! What each key means is the Presenter's; this View draws the form and turns its clicks into
//! Presenter events.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::{
    CheckboxView, DocumentField, DocumentFormEvent, DocumentFormPresenter, DocumentFormViewModel,
};
use lib_shell::Hint;
use lib_viper::view::{SelectList, TextFieldView};

use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::theme::color;

type Clicks = ModuleDialogClicks<DocumentFormPresenter>;

/// The Document type and Key date selects' list scroll windows.
#[derive(Default)]
pub struct FormWidgets {
    doc_type: usize,
    key_kind: usize,
}

/// The `lib-documents` Add and Edit document Presenter's [`DialogView`], hosted in a
/// `ModuleDialog` (ADR-0056).
impl DialogView for DocumentFormPresenter {
    type Widgets = FormWidgets;

    fn kind(view: &DocumentFormViewModel) -> &'static str {
        if view.editing { "edit" } else { "add" }
    }

    fn render(
        view: &DocumentFormViewModel,
        widgets: &mut FormWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(
            view,
            &mut widgets.doc_type,
            &mut widgets.key_kind,
            clicks,
            cx,
        )
    }

    fn readback(view: &DocumentFormViewModel) -> Vec<(&'static str, String)> {
        let mut values = Vec::new();
        if let Some(path) = &view.path {
            values.push(("path", path.text.clone()));
        }
        values.push(("document_title", view.title.text.clone()));
        values.push(("date", view.date.text.clone()));
        values.push((
            "error",
            view.path_error
                .clone()
                .or_else(|| view.date_error.clone())
                .unwrap_or_default(),
        ));
        framed(&view.frame, values)
    }

    fn hints(view: &DocumentFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// A click on a text field or the reminder focuses it; the reminder's also flips it.
fn click_field(field: DocumentField) -> Vec<DocumentFormEvent> {
    let mut events = vec![DocumentFormEvent::Focus(field)];
    if field == DocumentField::Reminder {
        events.push(DocumentFormEvent::ToggleReminder);
    }
    events
}

fn render(
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
            actions.send_all(click_field(which)),
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
                on_field_click: actions.send(DocumentFormEvent::ToggleList(field)),
                on_option_click,
            },
            cx,
        )
    };

    let types: Vec<_> = view
        .doc_type
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
    let kinds: Vec<_> = view
        .key_kind
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
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
            actions.option(move |index| {
                types
                    .get(index)
                    .map(|value| vec![DocumentFormEvent::Type(*value)])
                    .unwrap_or_default()
            }),
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
        actions.option(move |index| {
            kinds
                .get(index)
                .map(|value| vec![DocumentFormEvent::KeyKind(*value)])
                .unwrap_or_default()
        }),
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
            actions.send_all(click_field(DocumentField::Reminder)),
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
