//! Renders the **Delete unit** destructive-confirm dialog (issue #186's own "2d"), on the shared
//! `crate::dialog` chrome's destructive variant (`overlay(true, ...)`/`header(_, true)`).
//! References are named, and the unit's own code must be typed back exactly
//! (`DeleteUnitForm::is_valid`, case-sensitive) before the confirm button enables --
//! deletion can't happen by muscle memory.
//!
//! Reuses `form::fields::text_field` for the confirm input, but always `focused: true` with a
//! no-op click handler: unlike Code/Name in the Add/Edit dialogs, there's only one field here, so
//! nothing to click into or `Tab` between (`chrome::dialog_host::handle_key` swallows `Tab` for every
//! Dialog).
//!
//! The warning copy and the reference panel's own numbers ("1 account and 9 transactions",
//! "0.4120 u") are the ticket's own fixed mockup example, shown identically regardless of which
//! row is being deleted -- same reasoning as the Edit unit dialog's usage notice (issue #185):
//! no real accounts/transactions data backs any row in this map. The reference panel's own unit
//! *name* is real (`row.name`), since the ticket asks for "each affected dummy record" to be
//! actually named, not just the counts.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_ledger::UnitUse;
use lib_units::UnitRow;

use super::{Change, UnitsDialogHandlers};
use crate::{
    dialog,
    form::{
        fields::text_field,
        select_field::{self, SelectFieldProps},
    },
    settings::units::form::{DeleteUnitForm, NewBase},
    theme::color,
};

pub fn render(
    row: &UnitRow,
    form: &DeleteUnitForm,
    handlers: UnitsDialogHandlers,
    cx: &App,
) -> AnyElement {
    if let Some(uses) = form.in_use {
        return render_in_use(row, form, uses, handlers, cx);
    }
    let code = row.code.as_str();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_units_delete_title(code),
            true,
            cx,
        ))
        .child(dialog::body(fields(row, form, &handlers, cx)))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-unit-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-unit-confirm",
                    crate::msg::desktop_settings_units_delete_submit(),
                    form.is_valid(),
                    true,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, true, card, cx)
}

/// The refusal: a Unit that an Account, a Bill Plan or a Budget names can't be deleted, so the
/// Dialog says what names it and offers Deactivate on the neutral chrome. Deactivating loses
/// nothing, so it asks for no typed code; the base Unit can't be deactivated, so its confirm stays
/// disabled and the note says what to do first.
fn render_in_use(
    row: &UnitRow,
    form: &DeleteUnitForm,
    uses: UnitUse,
    handlers: UnitsDialogHandlers,
    cx: &App,
) -> AnyElement {
    let code = row.code.as_str();
    let part = |count: usize, message: fn(i64) -> String| {
        (count > 0).then(|| message(i64::try_from(count).unwrap_or(i64::MAX)))
    };
    let named = [
        part(
            uses.accounts,
            crate::msg::desktop_settings_units_in_use_accounts,
        ),
        part(
            uses.bill_plans,
            crate::msg::desktop_settings_units_in_use_bill_plans,
        ),
        part(
            uses.budgets,
            crate::msg::desktop_settings_units_in_use_budgets,
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let note = |text: String| {
        div()
            .text_size(px(11.5))
            .text_color(color::muted(cx))
            .child(text)
            .into_any_element()
    };
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(crate::msg::desktop_settings_units_in_use(code, &named))
            .into_any_element(),
        note(crate::msg::desktop_settings_units_in_use_help()),
    ];
    if form.is_base {
        fields.push(note(crate::msg::desktop_settings_units_in_use_base()));
    }
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_units_deactivate_title(code),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-unit-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-unit-confirm",
                    crate::msg::desktop_settings_units_deactivate_submit(),
                    form.is_valid(),
                    false,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

fn fields(
    row: &UnitRow,
    form: &DeleteUnitForm,
    handlers: &UnitsDialogHandlers,
    cx: &App,
) -> Vec<AnyElement> {
    let code = row.code.as_str();
    let mut fields = vec![
        warning_copy().into_any_element(),
        dialog::info_panel(
            crate::msg::desktop_settings_units_delete_held(&row.name, "0.4120 u"),
            cx,
        )
        .into_any_element(),
    ];
    if let Some(new_base) = &form.new_base {
        fields.push(new_base_field(code, new_base, handlers, cx));
    }
    fields.push(confirm_field(code, form.confirm_input.text(), cx).into_any_element());
    fields
}

/// The select over the Units that stay, shown only for the base Unit. The confirm input owns the
/// typing, so this select is never the focused field; `Up`/`Down` and a click drive it.
fn new_base_field(
    code: &str,
    new_base: &NewBase,
    handlers: &UnitsDialogHandlers,
    cx: &App,
) -> AnyElement {
    let on_change = handlers.on_change.clone();
    let on_field_click: dialog::OnClick = {
        let on_change = on_change.clone();
        Rc::new(move |window, cx| on_change(Change::NewBaseField, window, cx))
    };
    let on_option_click: select_field::OnOptionClick =
        Rc::new(move |index, window, cx| on_change(Change::NewBaseOption(index), window, cx));
    div()
        .child(select_field::render(
            SelectFieldProps {
                id: "delete-unit-new-base",
                label: crate::msg::desktop_settings_units_delete_new_base_label().into(),
                options: &new_base.options(),
                state: &new_base.select,
                focused: false,
                accent: false,
                read_only: None,
                on_field_click,
                on_option_click,
            },
            cx,
        ))
        .child(
            div()
                .mt(px(6.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_units_delete_new_base_help(
                    code,
                )),
        )
        .into_any_element()
}

fn warning_copy() -> impl IntoElement {
    div()
        .text_size(px(13.0))
        .child(crate::msg::desktop_settings_units_delete_warning(1, 9))
}

fn confirm_field(code: &str, value: &str, cx: &App) -> impl IntoElement {
    text_field(
        "delete-unit-confirm-input",
        crate::msg::desktop_settings_units_delete_confirm_label(code),
        value,
        code,
        true,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
        cx,
    )
}
