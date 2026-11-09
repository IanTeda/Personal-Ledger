//! **Edit extracted facts** on the shared `crate::dialog` frame: an Unfiled Document's merchant,
//! date, total and Document Type. What each key means is the
//! Presenter's; this View draws the form and turns its clicks into Presenter events.

use gpui::{AnyElement, App, div, prelude::*};
use lib_documents::presenter::{
    FactsField, FactsFormEvent, FactsFormPresenter, FactsFormViewModel,
};
use lib_shell::Hint;
use lib_viper::view::TextFieldView;

use super::document_form::with_error;
use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

type Clicks = ModuleDialogClicks<FactsFormPresenter>;

/// The `lib-documents` Edit extracted facts Presenter's [`DialogView`], hosted in a
/// `ModuleDialog` (ADR-0056). Its widget state is the Document type list's scroll window.
impl DialogView for FactsFormPresenter {
    type Widgets = usize;

    fn kind(_view: &FactsFormViewModel) -> &'static str {
        "facts"
    }

    fn render(
        view: &FactsFormViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    fn readback(view: &FactsFormViewModel) -> Vec<(&'static str, String)> {
        framed(
            &view.frame,
            vec![
                ("merchant", view.merchant.text.clone()),
                ("total", view.total.text.clone()),
                (
                    "error",
                    view.total_error
                        .clone()
                        .or_else(|| view.date_error.clone())
                        .unwrap_or_default(),
                ),
            ],
        )
    }

    fn hints(view: &FactsFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// A click on a field focuses it; the type select's toggles its list.
fn click_field(field: FactsField) -> FactsFormEvent {
    if field == FactsField::Type {
        FactsFormEvent::ToggleTypeList
    } else {
        FactsFormEvent::Focus(field)
    }
}

fn render(view: &FactsFormViewModel, scroll: &mut usize, actions: &Clicks, cx: &App) -> AnyElement {
    let text = |id: &'static str, field: &TextFieldView, which: FactsField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.send(click_field(which)),
            cx,
        )
    };
    let types: Vec<_> = view
        .doc_type
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
    let type_field = select_field::render(
        SelectFieldProps {
            id: "facts-type".into(),
            label: view.doc_type.label.clone().into(),
            options: &labels(&view.doc_type),
            state: &drawn(&view.doc_type, view.doc_type_list, scroll),
            focused: view.doc_type.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.send(click_field(FactsField::Type)),
            on_option_click: actions.option(move |index| {
                types
                    .get(index)
                    .map(|value| vec![FactsFormEvent::Type(*value)])
                    .unwrap_or_default()
            }),
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
