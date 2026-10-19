//! Renders the **Add unit** dialog (issue #184's own "2b", issue #715's PRICE SOURCE group), on
//! the shared `crate::dialog` chrome. `Shell`'s Dialog host holds the live form state
//! (`super::super::form::UnitForm`) and takes every keystroke while it's open
//! (`InputMode::Dialog`) -- this module is a pure render-helper. The fields are the same as Edit
//! unit's ([`super::form_fields`]): this is 16c·src's form, empty, with Source on Manual entry and
//! **Add** as the confirm.

use gpui::{AnyElement, App, div, prelude::*};

use super::{FieldIds, UnitsDialogHandlers, form_fields};
use crate::{dialog, settings::units::form::UnitForm};

const IDS: FieldIds = FieldIds {
    code: "add-unit-code",
    kind: "add-unit-type",
    name: "add-unit-name",
    source: "add-unit-source",
    symbol: "add-unit-symbol",
    quote: "add-unit-quote",
    test: "add-unit-test",
};

pub fn render(form: &UnitForm, handlers: UnitsDialogHandlers, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_units_add_title(),
            false,
            cx,
        ))
        .child(dialog::body(form_fields(IDS, form, &handlers, cx)))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-unit-cancel", handlers.on_cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "add-unit-confirm",
                    crate::msg::desktop_settings_units_add_submit(),
                    form.is_valid(),
                    false,
                    handlers.on_confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, false, card, cx)
}
