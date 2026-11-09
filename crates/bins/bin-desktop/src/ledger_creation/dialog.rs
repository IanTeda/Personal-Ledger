//! The Create ledger Dialog: the chosen file's name (read-only), then Code and Type, Name and
//! Precision, as Add unit draws them. An Entity in the Dialog host
//! (`shell::view::dialog_entity`); every Dialog command and typed key reaches the Presenter as a
//! `lib-viper` `DialogInput` and every click as a Presenter event.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*};
use lib_ledger_creation::presenter::{
    CreateLedgerEvent, CreateLedgerPresenter, CreateLedgerViewModel, LedgerField,
};
use lib_shell::Hint;
use lib_viper::view::SelectList;

use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};
use crate::units::help;

type Clicks = ModuleDialogClicks<CreateLedgerPresenter>;

/// The Type select's open-list scroll window, the widget state the View keeps beside the
/// Presenter.
#[derive(Default)]
pub struct CreateLedgerWidgets {
    scroll: usize,
}

/// `lib-ledger-creation`'s [`CreateLedgerPresenter`] hosted in a `ModuleDialog`: a confirmed
/// create lands on the root Router, which opens the new Ledger.
impl DialogView for CreateLedgerPresenter {
    type Widgets = CreateLedgerWidgets;

    fn kind(_view: &CreateLedgerViewModel) -> &'static str {
        "CreateLedger"
    }

    fn render(
        view: &CreateLedgerViewModel,
        widgets: &mut CreateLedgerWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, &mut widgets.scroll, clicks, cx)
    }

    fn readback(view: &CreateLedgerViewModel) -> Vec<(&'static str, String)> {
        let focused = [
            (view.code.focused, "Code"),
            (view.kind.focused, "Type"),
            (view.name.focused, "Name"),
            (view.precision.focused, "Precision"),
        ]
        .into_iter()
        .find_map(|(focused, name)| focused.then(|| name.to_string()))
        .unwrap_or_default();
        vec![
            ("title", view.frame.title.clone()),
            ("file", view.file.text.clone()),
            ("code", view.code.text.clone()),
            ("name", view.name.text.clone()),
            ("precision", view.precision.text.clone()),
            ("kind", chosen_label(&view.kind).unwrap_or_default()),
            ("kinds", labels(&view.kind).join("|")),
            ("field", focused),
            ("select_open", view.list.is_open().to_string()),
            ("can_confirm", view.frame.can_confirm.to_string()),
        ]
    }

    /// The Dialog has no hints of its own and leaves the View's on the status line.
    fn hints(_view: &CreateLedgerViewModel) -> Vec<Hint> {
        Vec::new()
    }
}

fn render(
    view: &CreateLedgerViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let text = |id: &'static str, field: &lib_viper::view::TextFieldView, to: LedgerField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.send(CreateLedgerEvent::Focus(to)),
            cx,
        )
    };
    let list = if view.kind.focused {
        view.list
    } else {
        SelectList::default()
    };
    let options = labels(&view.kind);
    let kind = select_field::render(
        SelectFieldProps {
            id: "create-ledger-type".into(),
            label: view.kind.label.clone().into(),
            options: &options,
            state: &drawn(&view.kind, list, scroll),
            focused: view.kind.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.send(CreateLedgerEvent::ToggleKind),
            on_option_click: actions.option({
                let kinds: Vec<_> = view
                    .kind
                    .options
                    .iter()
                    .map(|choice| choice.value)
                    .collect();
                move |index| {
                    kinds
                        .get(index)
                        .map(|kind| vec![CreateLedgerEvent::Kind(*kind)])
                        .unwrap_or_default()
                }
            }),
        },
        cx,
    );
    let file = text_field(
        "create-ledger-file",
        label(view.file.label.clone()),
        &view.file.text,
        "",
        false,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
        cx,
    );
    let body = vec![
        file,
        div().child(view.help.clone()).into_any_element(),
        two_up([
            text("create-ledger-code", &view.code, LedgerField::Code),
            kind,
        ]),
        div()
            .child(two_up([
                text("create-ledger-name", &view.name, LedgerField::Name),
                text(
                    "create-ledger-precision",
                    &view.precision,
                    LedgerField::Precision,
                ),
            ]))
            .child(help(view.precision_help.clone(), cx))
            .into_any_element(),
    ];
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("create-ledger-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "create-ledger-confirm",
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
