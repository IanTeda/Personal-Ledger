//! The Delete unit Dialog (16d) in the red treatment, and the Deactivate unit Dialog it turns into
//! for a Unit in use, as it opens. The typed-back code always holds the typing, so
//! the base Unit's New base unit select is driven by `Up`/`Down` and the mouse.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::presenter::{DeleteUnitEvent, DeleteUnitPresenter, DeleteUnitViewModel};

use super::{
    Shared, UnitsDialog, View, choose_option, enter, help, refusal, select_key, send, synced,
    toggle_select, typed,
};
use crate::{
    chrome::dialog_host::{DialogActions, DialogKey, DialogOutcome},
    dialog,
    form::{
        fields::text_field,
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    theme::color,
};

fn handle(presenter: &Shared<DeleteUnitPresenter>, event: DeleteUnitEvent) {
    send(presenter, event, DeleteUnitPresenter::handle);
}

pub(super) fn key(
    presenter: &Shared<DeleteUnitPresenter>,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.borrow().view_model();
    if let Some(new_base) = &view.new_base {
        let select_key_now = match key {
            DialogKey::Up | DialogKey::Down => true,
            DialogKey::Enter => select.is_open(),
            _ => false,
        };
        if select_key_now {
            let (_, value) = select_key(select, &new_base.select, key);
            if let Some(id) = value {
                handle(presenter, DeleteUnitEvent::NewBase(id));
            }
            return DialogOutcome::Handled;
        }
    }
    match key {
        DialogKey::Enter | DialogKey::CtrlEnter => enter(view.frame.can_confirm),
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => match view
            .confirm
            .as_ref()
            .and_then(|confirm| typed(&confirm.text, key))
        {
            Some(text) => {
                handle(presenter, DeleteUnitEvent::ConfirmText(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

fn click_new_base(dialog: &mut UnitsDialog) {
    if let View::DeleteUnit(presenter, select) = &mut dialog.view
        && let Some(new_base) = presenter.borrow().view_model().new_base
    {
        toggle_select(select, &new_base.select);
    }
}

fn click_new_base_option(dialog: &mut UnitsDialog, index: usize) {
    if let View::DeleteUnit(presenter, select) = &mut dialog.view {
        let chosen = presenter
            .borrow()
            .view_model()
            .new_base
            .and_then(|new_base| choose_option(select, &new_base.select, index));
        if let Some(id) = chosen {
            handle(presenter, DeleteUnitEvent::NewBase(id));
        }
    }
}

pub(super) fn render(
    view: &DeleteUnitViewModel,
    select: &SelectState,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let note = |text: String| {
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    let mut fields = Vec::new();
    if let Some(in_use) = &view.in_use {
        fields.push(
            div()
                .debug_selector(|| "delete-unit-in-use".to_string())
                .text_size(px(13.0))
                .child(in_use.text.clone())
                .into_any_element(),
        );
        fields.push(note(in_use.help.clone()));
        fields.extend(in_use.base.clone().map(|base| {
            div()
                .debug_selector(|| "delete-unit-base-note".to_string())
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(base)
                .into_any_element()
        }));
    }
    if let Some(new_base) = &view.new_base {
        let options = super::labels(&new_base.select);
        fields.push(
            div()
                .child(select_field::render(
                    SelectFieldProps {
                        id: "delete-unit-new-base",
                        label: new_base.select.label.clone().into(),
                        options: &options,
                        state: &synced(select, &new_base.select),
                        focused: false,
                        accent: false,
                        read_only: None,
                        on_field_click: UnitsDialog::on_click(actions, click_new_base),
                        on_option_click: UnitsDialog::on_option(actions, click_new_base_option),
                    },
                    cx,
                ))
                .child(help(new_base.help.clone(), cx))
                .into_any_element(),
        );
    }
    if let Some(confirm) = &view.confirm {
        fields.push(
            text_field(
                "delete-unit-confirm-input",
                confirm.label.clone(),
                &confirm.text,
                &confirm.placeholder,
                confirm.focused,
                Rc::new(|_window: &mut Window, _cx: &mut App| {}),
                cx,
            )
            .into_any_element(),
        );
    }
    fields.extend(refusal(view.refusal.clone(), cx));

    let destructive = view.frame.destructive;
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-unit-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-unit-confirm",
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

pub(super) fn readback(
    view: &DeleteUnitViewModel,
    select: &SelectState,
) -> Vec<(&'static str, String)> {
    vec![
        (
            "confirm",
            view.confirm
                .as_ref()
                .map(|confirm| confirm.text.clone())
                .unwrap_or_default(),
        ),
        ("in_use", view.in_use.is_some().to_string()),
        (
            "in_use_text",
            view.in_use
                .as_ref()
                .map(|in_use| in_use.text.clone())
                .unwrap_or_default(),
        ),
        (
            "new_base",
            view.new_base
                .as_ref()
                .and_then(|new_base| super::chosen_label(&new_base.select))
                .unwrap_or_default(),
        ),
        (
            "new_base_options",
            view.new_base
                .as_ref()
                .map(|new_base| super::labels(&new_base.select).join("|"))
                .unwrap_or_default(),
        ),
        ("select_open", select.is_open().to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
