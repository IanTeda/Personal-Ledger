//! Renders the **Add unit** dialog (issue #184's own "2b"), on the shared `crate::dialog`
//! chrome. `Shell` owns the live form state (`settings::UnitForm`) and every keystroke while
//! it's open (`InputMode::Dialog`, `Shell::handle_dialog_key`) -- this module is a pure
//! render-helper, the same split every other `view::settings` submodule already uses.
//!
//! `field_click`/`text_field`/`type_field` are `pub(super)`: the Edit unit dialog (issue #185)
//! wraps the same `UnitForm` and reuses them verbatim rather than duplicating this module's own
//! field rendering -- see `super::edit_unit_dialog`'s own doc.
//!
//! Code/Name are this crate's **first real editable text fields** -- every other "field" built
//! so far (`general::field_value`) is deliberately static per that module's own doc, since
//! nothing needed to change it yet. There's still no real text cursor or selection here: a
//! clicked field becomes "focused" (its border turns `ACCENT`, and a trailing `\u{2502}` caret
//! shows), typed characters append to the end, `Backspace` pops the last one -- append/pop-only
//! editing, not an arbitrary-position cursor, is enough for a single-line Code/Name field with no
//! existing value to edit into the middle of.
//!
//! Type is a segmented control (`super::ledger_units::segmented_control`, made `pub(super)` for
//! this), not the raw mockup markup's own `<select>` -- see this ticket's own resolution comment
//! for why (a real dropdown has no more precedent in this crate than a real text cursor does, and
//! unlike Code/Name, Type can't fall back to "static until a future ticket needs it" the way
//! `general`'s fields do, since a brand-new unit needs a real value here). The mockup's own
//! `<select>` options ("currency / crypto / etf / stock") also don't match the ticket's own body
//! ("currency / cryptocurrency / custom") -- built to the ticket's wording, the operative spec
//! here, not the mockup's own copy.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    dialog,
    settings::{AddUnitField, UnitForm, UnitKind},
    theme::color,
};

use super::{field_label, ledger_units::segmented_control};

pub type OnFieldClick = Rc<dyn Fn(AddUnitField, &mut Window, &mut App)>;
pub type OnKindClick = Rc<dyn Fn(UnitKind, &mut Window, &mut App)>;
pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

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
        .child(dialog::header("Add unit", false))
        .child(dialog::body([
            text_field(
                "add-unit-code",
                "Code",
                &form.code,
                "e.g. usd",
                form.focused_field == AddUnitField::Code,
                field_click(AddUnitField::Code, on_field_click.clone()),
            )
            .into_any_element(),
            text_field(
                "add-unit-name",
                "Name",
                &form.name,
                "e.g. US Dollar",
                form.focused_field == AddUnitField::Name,
                field_click(AddUnitField::Name, on_field_click),
            )
            .into_any_element(),
            type_field("add-unit-type", form.kind, on_kind_click).into_any_element(),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("add-unit-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "add-unit-confirm",
                "Add",
                form.is_valid(),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(false, card)
}

/// Curries `field` into a plain click handler -- what [`text_field`] binds its own `on_click` to.
pub(super) fn field_click(field: AddUnitField, on_click: OnFieldClick) -> dialog::OnClick {
    Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
}

/// One Code/Name field: `super::field_label` above a clickable value box mirroring
/// `super::field_value`'s own border/padding, plus the focused-state border and caret this
/// crate's first real text input needs (see the module doc).
pub(super) fn text_field(
    id: &'static str,
    label: &'static str,
    value: &str,
    placeholder: &'static str,
    focused: bool,
    on_click: dialog::OnClick,
) -> impl IntoElement {
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::INK_TERTIARY,
        )
    } else {
        (SharedString::from(format!("{value}{caret}")), color::INK)
    };

    div().child(field_label(label)).child(
        div()
            .id(id)
            .cursor_pointer()
            .w_full()
            .py(px(8.0))
            .px(px(10.0))
            .border_1()
            .border_color(if focused {
                color::ACCENT
            } else {
                color::BORDER
            })
            .text_size(px(13.0))
            .text_color(text_color)
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(text),
    )
}

/// The Type field: `super::field_label` above a segmented control (see the module doc for why
/// this isn't the raw markup's own `<select>`). `id_prefix` keeps the Add/Edit dialogs' own
/// segmented controls element-id-distinct, even though only one is ever mounted at a time.
pub(super) fn type_field(
    id_prefix: &'static str,
    selected: UnitKind,
    on_click: OnKindClick,
) -> impl IntoElement {
    div().child(field_label("Type")).child(segmented_control(
        id_prefix,
        &UnitKind::ALL,
        selected,
        UnitKind::label,
        on_click,
    ))
}
