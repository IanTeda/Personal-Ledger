//! Renders the **Add institution** dialog (issue #187's own "2e"), on the shared `crate::dialog`
//! chrome -- the one dialog in this map whose own raw markup sets `width:460px` instead of the
//! shared [`dialog::WIDTH`] (see that constant's own doc).
//!
//! Three field shapes, none reused verbatim from the Unit dialogs:
//! - **Institution name**: a real text field, but the dialog's only one -- reuses
//!   `add_unit_dialog::text_field` with `focused: true` and a no-op click handler, the same
//!   "implicitly focused, nothing to click into" shape `delete_unit_dialog`'s own confirm field
//!   uses.
//! - **Account types**: multi-select chips (`docs/ux/desktop/Shell & Navigation/styles.css`'s
//!   `.chip` role) -- the first *multi*-select control in this crate; every other "pick one"
//!   control (segmented controls, dot radios) only ever holds one selection at a time.
//! - **Default unit**: a single-select built from `Shell::settings_units`' own live `Vec`, not a
//!   compile-time enum -- `ledger_units::segmented_control`'s `&'static [T]` bound can't take a
//!   runtime slice, so this module builds its own segmented-style row instead of reusing that
//!   function. Still a segmented control rather than the raw mockup's own `<select>`, same
//!   reasoning as `add_unit_dialog`'s own Type field.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    dialog,
    settings::{AccountType, AddInstitutionForm, UnitRow},
    theme::color,
};

use super::{add_unit_dialog::text_field, field_label};

const WIDTH: gpui::Pixels = px(460.0);

pub type OnAccountTypeClick = Rc<dyn Fn(AccountType, &mut Window, &mut App)>;
/// Passes the clicked unit's own code -- `Shell::settings_units` is a live `Vec` the dialog
/// doesn't own, so an index would need re-deriving against it anyway; the code is what
/// `AddInstitutionForm::default_unit_code` actually stores (see that field's own doc).
pub type OnUnitClick = Rc<dyn Fn(String, &mut Window, &mut App)>;
pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

pub fn render(
    form: &AddInstitutionForm,
    units: &[UnitRow],
    on_account_type_click: OnAccountTypeClick,
    on_unit_click: OnUnitClick,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_institutions_add_title(),
            false,
        ))
        .child(dialog::body([
            name_field(&form.name).into_any_element(),
            account_types_field(&form.account_types, on_account_type_click).into_any_element(),
            default_unit_field(units, form.default_unit_code.as_deref(), on_unit_click)
                .into_any_element(),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("add-institution-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "add-institution-confirm",
                crate::msg::desktop_settings_institutions_add_submit(),
                form.is_valid(),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(WIDTH, false, card)
}

fn name_field(value: &str) -> AnyElement {
    let placeholder = crate::msg::desktop_settings_institutions_name_placeholder();
    text_field(
        "add-institution-name",
        crate::msg::desktop_settings_institutions_name_label(),
        value,
        &placeholder,
        true,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
    )
    .into_any_element()
}

/// The `.chip` multi-select row (`docs/ux/desktop/Settings/README.md`'s Dialog components:
/// `display:flex; align-items:center; gap:6px; padding:6px 10px; border:1px solid
/// rgba(32,30,29,.30); font-size:12px; cursor:pointer`, selected takes the dark treatment).
fn account_types_field(selected: &[AccountType], on_click: OnAccountTypeClick) -> impl IntoElement {
    div()
        .child(field_label(
            crate::msg::desktop_settings_institutions_account_types(),
        ))
        .child(div().flex().flex_wrap().gap(px(8.0)).mt(px(4.0)).children(
            AccountType::ALL.into_iter().map(|account_type| {
                let checked = selected.contains(&account_type);
                let on_click = on_click.clone();
                chip(account_type, checked, on_click)
            }),
        ))
}

fn chip(
    account_type: AccountType,
    checked: bool,
    on_click: OnAccountTypeClick,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!(
            "add-institution-chip-{}",
            format!("{account_type:?}").to_lowercase()
        )))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(6.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::BORDER)
        .text_size(px(12.0))
        .when(checked, |this| {
            this.bg(color::INK).text_color(color::INK_ON_DARK)
        })
        .on_click(move |_event, window, cx| on_click(account_type, window, cx))
        .child(account_type.label())
}

/// The Default unit field: a segmented-style row built from a runtime `&[UnitRow]` (see the
/// module doc for why `ledger_units::segmented_control` doesn't fit here). Falls back to a plain
/// "No units available" note if `units` is empty -- `AddInstitutionForm::is_valid` already keeps
/// the Add button disabled in that case, so this is purely the field's own empty-state message.
fn default_unit_field(
    units: &[UnitRow],
    selected_code: Option<&str>,
    on_click: OnUnitClick,
) -> impl IntoElement {
    div()
        .child(field_label(
            crate::msg::desktop_settings_institutions_default_unit(),
        ))
        .child(if units.is_empty() {
            div()
                .text_size(px(13.0))
                .text_color(color::INK_TERTIARY)
                .child(crate::msg::desktop_settings_institutions_no_units())
                .into_any_element()
        } else {
            div()
                .flex()
                .border_1()
                .border_color(color::DIVIDER)
                .children(units.iter().enumerate().map(|(index, unit)| {
                    let selected = Some(unit.code.as_str()) == selected_code;
                    let on_click = on_click.clone();
                    let code = unit.code.clone();
                    div()
                        .id(SharedString::from(format!("add-institution-unit-{index}")))
                        .cursor_pointer()
                        .py(px(7.0))
                        .px(px(12.0))
                        .text_size(px(13.0))
                        .when(index > 0, |this| {
                            this.border_l(px(1.0)).border_color(color::DIVIDER)
                        })
                        .when(selected, |this| {
                            this.bg(color::ACCENT).text_color(color::INK_ON_DARK)
                        })
                        .on_click(move |_event, window, cx| on_click(code.clone(), window, cx))
                        .child(unit.code.clone())
                }))
                .into_any_element()
        })
}
