//! Renders the **Add unit** dialog (issue #184's own "2b"), on the shared `crate::dialog`
//! chrome. `Shell`'s Dialog host holds the live form state (`super::super::form::UnitForm`) and
//! takes every keystroke while it's open (`InputMode::Dialog`) -- this module is a pure
//! render-helper.
//!
//! `type_field` is `pub(super)`: the Edit unit dialog (issue #185) wraps the same `UnitForm` and
//! reuses it verbatim rather than duplicating this module's own field rendering -- see
//! `super::edit`'s own doc.
//!
//! Code/Name are this crate's **first real editable text fields** -- every other "field" built
//! so far (`form::fields::field_value`) is deliberately static per that module's own doc, since
//! nothing needed to change it yet. There's still no real text cursor or selection here: a
//! clicked field becomes "focused" (its border turns `ACCENT`, and a trailing `\u{2502}` caret
//! shows), typed characters append to the end, `Backspace` pops the last one -- append/pop-only
//! editing, not an arbitrary-position cursor, is enough for a single-line Code/Name field with no
//! existing value to edit into the middle of.
//!
//! Type is a segmented control (`segmented_control`, below), not the raw mockup markup's own
//! `<select>` -- see this ticket's own resolution comment for why (a real dropdown has no more
//! precedent in this crate than a real text cursor does, and unlike Code/Name, Type can't fall
//! back to "static until a future ticket needs it" the way `general`'s fields do, since a
//! brand-new unit needs a real value here). The mockup's own `<select>` options ("currency /
//! crypto / etf / stock") also don't match the ticket's own body ("currency / cryptocurrency /
//! custom") -- built to the ticket's wording, the operative spec here, not the mockup's own
//! copy. `segmented_control` itself moved here from the now-deleted `ledger_units.rs` (issue
//! #189): this is its only consumer once that module's own two fields were removed.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*};
use lib_locale::Label;
use lib_units::UnitKind;

use super::{Change, UnitsDialogHandlers, on_change};
use crate::{
    dialog,
    form::fields::{field_label, segmented_control, text_field},
    settings::units::form::{AddUnitField, UnitForm},
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
        .child(dialog::body([
            text_field(
                "add-unit-code",
                lib_locale::msg::column_code(),
                form.code.text(),
                &crate::msg::desktop_settings_units_code_placeholder(),
                form.focused_field == AddUnitField::Code,
                on_change(&handlers, Change::Field(AddUnitField::Code)),
                cx,
            )
            .into_any_element(),
            text_field(
                "add-unit-name",
                lib_locale::msg::column_name(),
                form.name.text(),
                &crate::msg::desktop_settings_units_name_placeholder(),
                form.focused_field == AddUnitField::Name,
                on_change(&handlers, Change::Field(AddUnitField::Name)),
                cx,
            )
            .into_any_element(),
            type_field("add-unit-type", form.kind, &handlers, cx).into_any_element(),
        ]))
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

/// The Type field: [`field_label`] above a segmented control (see the module doc for why
/// this isn't the raw markup's own `<select>`). `id_prefix` keeps the Add/Edit dialogs' own
/// segmented controls element-id-distinct, even though only one is ever mounted at a time.
pub(super) fn type_field(
    id_prefix: &'static str,
    selected: UnitKind,
    handlers: &UnitsDialogHandlers,
    cx: &App,
) -> impl IntoElement {
    let on_change = handlers.on_change.clone();
    let on_kind_click = Rc::new(move |kind, window: &mut Window, cx: &mut App| {
        on_change(Change::Kind(kind), window, cx)
    });
    div()
        .child(field_label(lib_locale::msg::column_type()))
        .child(segmented_control(
            id_prefix,
            UnitKind::all(),
            selected,
            |kind| kind.label(),
            on_kind_click,
            cx,
        ))
}
