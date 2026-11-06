//! The Create ledger Dialog: the chosen file's name (read-only), then Code and Type, Name and
//! Precision, as Add unit draws them. An Entity in the Dialog host
//! (`shell::view::dialog_entity`); every Dialog command and typed key reaches the Presenter as a
//! `lib-viper` `DialogInput` and every click as a Presenter event.

use std::rc::Rc;

use gpui::{AnyElement, App, Context, EventEmitter, IntoElement, Render, Window, div, prelude::*};
use lib_ledger_creation::interactor::LedgerCreated;
use lib_ledger_creation::presenter::{
    CreateLedgerEvent, CreateLedgerPresenter, CreateLedgerViewModel, LedgerField,
};
use lib_viper::dialog::DialogInput;
use lib_viper::view::SelectList;

use crate::dialog;
use crate::form::dialog_fields::{label, text_field, two_up};
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{chosen_label, drawn, labels};
use crate::shell::view::dialog_entity::{
    DialogClicks, DialogClosed, DialogReadback, HostRouter, HostedDialog,
};
use crate::units::help;

type Clicks = DialogClicks<CreateLedgerDialog>;

/// A create went through: the Ledger file now exists at `created.path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerCreatedEvent(pub LedgerCreated);

pub struct CreateLedgerDialog {
    presenter: CreateLedgerPresenter,
    /// The Type select's open-list scroll window.
    scroll: usize,
}

impl CreateLedgerDialog {
    pub fn new(presenter: CreateLedgerPresenter) -> Self {
        Self {
            presenter,
            scroll: 0,
        }
    }

    fn send(&mut self, event: CreateLedgerEvent, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.handle(event, &mut router);
        match router.closed() {
            Some(Some(created)) => {
                cx.emit(LedgerCreatedEvent(created));
                cx.emit(DialogClosed);
            }
            Some(None) => cx.emit(DialogClosed),
            None => {}
        }
    }

    /// An event that never closes the Dialog: a click on a field or a list row.
    fn apply(&mut self, event: CreateLedgerEvent) {
        self.presenter.handle(event, &mut HostRouter::default());
    }

    fn click_option(&mut self, index: usize) {
        let kind = self
            .presenter
            .view_model()
            .kind
            .options
            .get(index)
            .map(|choice| choice.value);
        if let Some(kind) = kind {
            self.apply(CreateLedgerEvent::Kind(kind));
        }
    }
}

impl EventEmitter<DialogClosed> for CreateLedgerDialog {}
impl EventEmitter<LedgerCreatedEvent> for CreateLedgerDialog {}

impl HostedDialog for CreateLedgerDialog {
    fn input(&mut self, input: DialogInput, cx: &mut Context<'_, Self>) {
        self.send(CreateLedgerEvent::Input(input), cx);
    }

    fn can_confirm(&self) -> bool {
        self.presenter.view_model().frame.can_confirm
    }

    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.send(CreateLedgerEvent::Confirm, cx);
    }

    /// A cancelled create has written nothing.
    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.send(CreateLedgerEvent::Cancel, cx);
    }

    fn close_open_select(&mut self) -> bool {
        let open = self.presenter.view_model().list.is_open();
        self.apply(CreateLedgerEvent::CloseList);
        open
    }

    fn readback(&self, _cx: &App) -> DialogReadback {
        let view = self.presenter.view_model();
        let focused = [
            (view.code.focused, "Code"),
            (view.kind.focused, "Type"),
            (view.name.focused, "Name"),
            (view.precision.focused, "Precision"),
        ]
        .into_iter()
        .find_map(|(focused, name)| focused.then(|| name.to_string()))
        .unwrap_or_default();
        DialogReadback {
            kind: "CreateLedger",
            values: vec![
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
            ],
        }
    }
}

impl Render for CreateLedgerDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let view = self.presenter.view_model();
        let clicks = Clicks::new(cx);
        render(&view, &mut self.scroll, &clicks, cx)
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
            actions.click(move |dialog: &mut CreateLedgerDialog| {
                dialog.apply(CreateLedgerEvent::Focus(to));
            }),
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
            on_field_click: actions.click(|dialog: &mut CreateLedgerDialog| {
                dialog.apply(CreateLedgerEvent::ToggleKind);
            }),
            on_option_click: actions.option(|dialog: &mut CreateLedgerDialog, index| {
                dialog.click_option(index);
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
