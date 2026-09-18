//! Renders the **Edit unit** dialog (issue #185's own "2c"), on the shared `crate::dialog`
//! chrome. Same form and field-editing rules as [`super::add_unit_dialog`] (see that module's
//! own doc) -- both wrap the same `settings::UnitForm`, so the only real differences here are
//! the dynamic title, the pre-filled starting values (`Shell::handle_unit_edit_click`, via
//! `UnitForm::from_row`), the usage-notice info panel, and the "Save" button label.
//!
//! The usage notice ("Used by 4 accounts · 604 transactions...") is the ticket's own fixed
//! mockup example, shown identically regardless of which row is being edited -- there's no real
//! accounts/transactions data behind *any* row in this map (see the map's own Destination), so a
//! per-row count would be inventing a fact this map has no data to back. Renamed to `save`
//! semantics: per the ticket's own body, a changed code just relabels the row here -- rewriting
//! real references is real-persistence work, out of scope.

use gpui::{AnyElement, div, prelude::*};

use crate::{
    dialog,
    settings::{AddUnitField, UnitForm},
};

use super::add_unit_dialog::{
    OnCancel, OnConfirm, OnFieldClick, OnKindClick, field_click, text_field, type_field,
};

const USAGE_NOTICE: &str = "Used by 4 accounts \u{b7} 604 transactions. Renaming is safe; changing the code rewrites references.";

pub fn render(
    form: &UnitForm,
    on_field_click: OnFieldClick,
    on_kind_click: OnKindClick,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            format!("Edit unit \u{2014} {}", form.code),
            false,
        ))
        .child(dialog::body([
            text_field(
                "edit-unit-code",
                "Code",
                &form.code,
                "e.g. usd",
                form.focused_field == AddUnitField::Code,
                field_click(AddUnitField::Code, on_field_click.clone()),
            )
            .into_any_element(),
            text_field(
                "edit-unit-name",
                "Name",
                &form.name,
                "e.g. US Dollar",
                form.focused_field == AddUnitField::Name,
                field_click(AddUnitField::Name, on_field_click),
            )
            .into_any_element(),
            type_field("edit-unit-type", form.kind, on_kind_click).into_any_element(),
            dialog::info_panel(USAGE_NOTICE).into_any_element(),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("edit-unit-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "edit-unit-confirm",
                "Save",
                form.is_valid(),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(false, card)
}
