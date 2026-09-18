//! Renders the **Delete unit** destructive-confirm dialog (issue #186's own "2d"), on the shared
//! `crate::dialog` chrome's destructive variant (`overlay(true, ...)`/`header(_, true)`).
//! References are named, and the unit's own code must be typed back exactly
//! (`settings::DeleteUnitForm::matches`, case-sensitive) before the confirm button enables --
//! deletion can't happen by muscle memory.
//!
//! Reuses `add_unit_dialog::text_field` for the confirm input, but always `focused: true` with a
//! no-op click handler: unlike Code/Name in the Add/Edit dialogs, there's only one field here, so
//! nothing to click into or `Tab` between (`Shell::handle_dialog_key`'s own `DeleteUnit` arm
//! swallows `Tab` for the same reason).
//!
//! The warning copy and the reference panel's own numbers ("1 account and 9 transactions",
//! "0.4120 u") are the ticket's own fixed mockup example, shown identically regardless of which
//! row is being deleted -- same reasoning as the Edit unit dialog's usage notice (issue #185):
//! no real accounts/transactions data backs any row in this map. The reference panel's own unit
//! *name* is real (`row.name`), since the ticket asks for "each affected dummy record" to be
//! actually named, not just the counts.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use crate::{
    dialog,
    settings::{DeleteUnitForm, UnitRow},
};

use super::add_unit_dialog::text_field;

pub fn render(
    row: &UnitRow,
    form: &DeleteUnitForm,
    on_cancel: dialog::OnClick,
    on_confirm: dialog::OnClick,
) -> AnyElement {
    let code = row.code.as_str();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(format!("Delete unit \u{2014} {code}"), true))
        .child(dialog::body([
            warning_copy().into_any_element(),
            dialog::info_panel(format!(
                "{name} \u{b7} 0.4120 u held in {name} account",
                name = row.name
            ))
            .into_any_element(),
            confirm_field(code, &form.confirm_input).into_any_element(),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("delete-unit-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "delete-unit-confirm",
                "Delete unit",
                form.matches(code),
                true,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(dialog::WIDTH, true, card)
}

fn warning_copy() -> impl IntoElement {
    div().text_size(px(13.0)).child(
        "This unit is referenced by 1 account and 9 transactions. Deleting it cannot be undone.",
    )
}

fn confirm_field(code: &str, value: &str) -> impl IntoElement {
    text_field(
        "delete-unit-confirm-input",
        format!("Type {code} to confirm"),
        value,
        code,
        true,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
    )
}
