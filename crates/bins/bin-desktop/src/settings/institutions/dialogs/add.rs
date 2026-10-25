//! Renders the **Add institution** dialog (issue #187's own "2e"), on the shared `crate::dialog`
//! frame -- the one dialog in this map whose own raw markup sets `width:460px` instead of the
//! shared [`dialog::WIDTH`] (see that constant's own doc).
//!
//! Three field shapes, none reused verbatim from the Unit dialogs:
//! - **Institution name**: a real text field, but the dialog's only one -- reuses
//!   `form::fields::text_field` with `focused: true` and a no-op click handler, the same
//!   "implicitly focused, nothing to click into" shape `units::dialogs::delete_unit`'s own confirm field
//!   uses.
//! - **Account types**: multi-select chips (`docs/ux/desktop-mockups/styles.css`'s
//!   `.chip` role) -- the first *multi*-select control in this crate; every other "pick one"
//!   control (segmented controls, dot radios) only ever holds one selection at a time.
//! - **Default unit**: a single-select built from `Root::settings_units`' own live `Vec`, not a
//!   compile-time enum -- `form::fields::segmented_control`'s `&'static [T]` bound can't take a
//!   runtime slice, so this module builds its own segmented-style row instead of reusing that
//!   function. Still a segmented control rather than the raw mockup's own `<select>`, same
//!   reasoning as `units::dialogs::unit_form`'s own Type field.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_institutions::InstitutionError;
use lib_units::UnitRow;

use super::{Change, InstitutionsDialogHandlers};
use crate::{
    dialog,
    form::colour_field::error_line,
    form::fields::{field_label, text_field},
    settings::institutions::form::{AddInstitutionForm, InstitutionProduct},
    theme::color,
};

const WIDTH: gpui::Pixels = px(460.0);

type OnAccountTypeClick = Rc<dyn Fn(InstitutionProduct, &mut Window, &mut App)>;
/// Passes the clicked unit's own code, which is what `AddInstitutionForm::default_unit_code`
/// stores.
type OnUnitClick = Rc<dyn Fn(String, &mut Window, &mut App)>;

pub fn render(
    form: &AddInstitutionForm,
    units: &[UnitRow],
    handlers: InstitutionsDialogHandlers,
    cx: &App,
) -> AnyElement {
    let on_account_type_click: OnAccountTypeClick = {
        let on_change = handlers.on_change.clone();
        Rc::new(move |account_type, window, cx| {
            on_change(Change::AccountType(account_type), window, cx)
        })
    };
    let on_unit_click: OnUnitClick = {
        let on_change = handlers.on_change.clone();
        Rc::new(move |code, window, cx| on_change(Change::Unit(code), window, cx))
    };
    let (on_cancel, on_confirm) = (handlers.on_cancel, handlers.on_confirm);
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_institutions_add_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            div()
                .child(name_field(form.name.text(), cx))
                .children(
                    form.name_error()
                        .map(|error| error_line(name_error_text(error), cx)),
                )
                .into_any_element(),
            account_types_field(&form.account_types, on_account_type_click, cx).into_any_element(),
            default_unit_field(units, form.default_unit_code.as_deref(), on_unit_click, cx)
                .into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-institution-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "add-institution-confirm",
                    crate::msg::desktop_settings_institutions_add_submit(),
                    form.is_valid(),
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}

/// A name refusal as the dialog words it.
fn name_error_text(error: &InstitutionError) -> String {
    match error {
        InstitutionError::DuplicateName(name) => {
            crate::msg::desktop_settings_institutions_error_name_taken(name)
        }
        InstitutionError::NameRequired => {
            crate::msg::desktop_settings_institutions_error_name_required()
        }
    }
}

fn name_field(value: &str, cx: &App) -> AnyElement {
    let placeholder = crate::msg::desktop_settings_institutions_name_placeholder();
    text_field(
        "add-institution-name",
        crate::msg::desktop_settings_institutions_name_label(),
        value,
        &placeholder,
        true,
        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
        cx,
    )
    .into_any_element()
}

/// The `.chip` multi-select row (`docs/ux/desktop-mockups/16-settings/README.md`'s Dialog components:
/// `display:flex; align-items:center; gap:6px; padding:6px 10px; border:1px solid
/// rgba(32,30,29,.30); font-size:12px; cursor:pointer`, selected takes the dark treatment).
fn account_types_field(
    selected: &[InstitutionProduct],
    on_click: OnAccountTypeClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .child(field_label(
            crate::msg::desktop_settings_institutions_account_types(),
        ))
        .child(div().flex().flex_wrap().gap(px(8.0)).mt(px(4.0)).children(
            InstitutionProduct::ALL.into_iter().map(|account_type| {
                let checked = selected.contains(&account_type);
                let on_click = on_click.clone();
                chip(account_type, checked, on_click, cx)
            }),
        ))
}

fn chip(
    account_type: InstitutionProduct,
    checked: bool,
    on_click: OnAccountTypeClick,
    cx: &App,
) -> impl IntoElement {
    let id = SharedString::from(format!(
        "add-institution-chip-{}",
        format!("{account_type:?}").to_lowercase()
    ));
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(6.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(12.0))
        .when(checked, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(move |_event, window, cx| on_click(account_type, window, cx))
        .child(account_type.label())
}

/// The Default unit field: a segmented-style row built from a runtime `&[UnitRow]` (see the
/// module doc for why `form::fields::segmented_control` doesn't fit here). Falls back to a plain
/// "No units available" note if `units` is empty -- `AddInstitutionForm::is_valid` already keeps
/// the Add button disabled in that case, so this is purely the field's own empty-state message.
fn default_unit_field(
    units: &[UnitRow],
    selected_code: Option<&str>,
    on_click: OnUnitClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .child(field_label(
            crate::msg::desktop_settings_institutions_default_unit(),
        ))
        .child(if units.is_empty() {
            div()
                .text_size(px(13.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_settings_institutions_no_units())
                .into_any_element()
        } else {
            div()
                .flex()
                .border_1()
                .border_color(color::divider(cx))
                .children(units.iter().enumerate().map(|(index, unit)| {
                    let selected = Some(unit.code.as_str()) == selected_code;
                    let on_click = on_click.clone();
                    let code = unit.code.clone();
                    div()
                        .id(SharedString::from(format!("add-institution-unit-{index}")))
                        .debug_selector(move || format!("add-institution-unit-{index}"))
                        .cursor_pointer()
                        .py(px(7.0))
                        .px(px(12.0))
                        .text_size(px(13.0))
                        .when(index > 0, |this| {
                            this.border_l(px(1.0)).border_color(color::divider(cx))
                        })
                        .when(selected, |this| {
                            this.bg(color::accent(cx)).text_color(color::background(cx))
                        })
                        .on_click(move |_event, window, cx| on_click(code.clone(), window, cx))
                        .child(unit.code.clone())
                }))
                .into_any_element()
        })
}
