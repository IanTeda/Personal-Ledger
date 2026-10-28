//! The Set base unit Dialog: one select over the active Units. `Up`/`Down` step it, `Space` opens
//! its list, and `Enter` commits an open list or confirms.

use gpui::{AnyElement, App, div, prelude::*};
use lib_units::presenter::{SetBaseUnitEvent, SetBaseUnitPresenter, SetBaseUnitViewModel};

use super::{
    Clicks, UnitsDialog, View, choose_option, enter, refusal, select_key, send, synced,
    toggle_select,
};
use crate::{
    dialog,
    form::{
        select::SelectState,
        select_field::{self, SelectFieldProps},
    },
    shell::view::dialog_host::{DialogKey, DialogOutcome},
};

fn handle(presenter: &mut SetBaseUnitPresenter, event: SetBaseUnitEvent) {
    send(presenter, event, SetBaseUnitPresenter::handle);
}

pub(super) fn key(
    presenter: &mut SetBaseUnitPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter if !select.is_open() => enter(view.frame.can_confirm),
        DialogKey::Up | DialogKey::Down | DialogKey::Enter | DialogKey::Char(' ') => {
            let (_, value) = select_key(select, &view.base, key);
            if let Some(id) = value {
                handle(presenter, SetBaseUnitEvent::Choose(id));
            }
            DialogOutcome::Handled
        }
        DialogKey::Tab | DialogKey::BackTab | DialogKey::Char(_) => DialogOutcome::Handled,
        _ => DialogOutcome::Ignored,
    }
}

fn click_field(dialog: &mut UnitsDialog) {
    if let View::SetBaseUnit(presenter, select) = &mut dialog.view {
        toggle_select(select, &presenter.view_model().base);
    }
}

fn click_option(dialog: &mut UnitsDialog, index: usize) {
    if let View::SetBaseUnit(presenter, select) = &mut dialog.view {
        let chosen = choose_option(select, &presenter.view_model().base, index);
        if let Some(id) = chosen {
            handle(presenter, SetBaseUnitEvent::Choose(id));
        }
    }
}

pub(super) fn render(
    view: &SetBaseUnitViewModel,
    select: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let options = super::labels(&view.base);
    let mut fields = vec![select_field::render(
        SelectFieldProps {
            id: "set-base-unit-select".into(),
            label: view.base.label.clone().into(),
            options: &options,
            state: &synced(select, &view.base),
            focused: view.base.focused,
            accent: false,
            read_only: None,
            on_field_click: UnitsDialog::on_click(actions, click_field),
            on_option_click: UnitsDialog::on_option(actions, click_option),
        },
        cx,
    )];
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("set-base-unit-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "set-base-unit-confirm",
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

pub(super) fn readback(view: &SetBaseUnitViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("base", super::chosen_label(&view.base).unwrap_or_default()),
        ("options", super::labels(&view.base).join("|")),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
