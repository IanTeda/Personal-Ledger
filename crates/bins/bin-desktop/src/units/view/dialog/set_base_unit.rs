//! The Set base unit Dialog: one select over the active Units, whose keys and list the Presenter
//! holds. `Up`/`Down` step it, `Space` opens its list, and `Enter` commits an open list or
//! confirms.

use gpui::{AnyElement, App, div, prelude::*};
use lib_units::presenter::{SetBaseUnitEvent, SetBaseUnitPresenter, SetBaseUnitViewModel};

use super::{Clicks, UnitsDialog, View, drawn, refusal, send};
use crate::{
    dialog,
    form::select_field::{self, SelectFieldProps},
};

fn handle(presenter: &mut SetBaseUnitPresenter, event: SetBaseUnitEvent) {
    send(presenter, event, SetBaseUnitPresenter::handle);
}

/// A click on the select's field: its list opens or closes.
fn click_field(dialog: &mut UnitsDialog) {
    if let View::SetBaseUnit(presenter, _) = &mut dialog.view {
        handle(presenter, SetBaseUnitEvent::ToggleList);
    }
}

/// A click on row `index` of the open list.
fn click_option(dialog: &mut UnitsDialog, index: usize) {
    if let View::SetBaseUnit(presenter, _) = &mut dialog.view {
        let chosen = presenter
            .view_model()
            .base
            .options
            .get(index)
            .map(|choice| choice.value);
        if let Some(id) = chosen {
            handle(presenter, SetBaseUnitEvent::Choose(id));
        }
    }
}

pub(super) fn render(
    view: &SetBaseUnitViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let options = super::labels(&view.base);
    let mut fields = vec![select_field::render(
        SelectFieldProps {
            id: "set-base-unit-select".into(),
            label: view.base.label.clone().into(),
            options: &options,
            state: &drawn(&view.base, view.list, scroll),
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
        ("select_open", view.list.is_open().to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
