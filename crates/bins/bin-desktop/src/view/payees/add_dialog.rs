//! Renders the **Add payee** and **Edit payee** dialogs (`docs/ux/desktop/20-payees/README.md`'s 6b
//! and 6c) on the shared `crate::dialog` chrome: Name, Default category (the shared `select_field`
//! dropdown, "none" then the leaf Categories), the shared Match rules field, and a callout. The two
//! differ only in title, rule placeholder, submit label and callout, so they share one renderer
//! keyed by [`PayeeDialogMode`]. `Shell` owns the live form (`payees::form::PayeeForm`) and every
//! keystroke while it is open.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::rules_field::{self, OnRemoveRule, RulesFieldProps};
use crate::{
    dialog,
    payees::{
        PayeeError,
        form::{PayeeField, PayeeForm, PayeeOptions},
    },
    theme::color,
    view::accounts::{
        add_dialog::{label, text_field},
        select_field::{self, SelectFieldProps},
    },
};

/// Which dialog to draw.
#[derive(Debug, Clone, Copy)]
pub enum PayeeDialogMode<'a> {
    Add,
    /// Editing the Payee stored as `name` (the title keeps it while the Name field is retyped),
    /// which `splits` Splits carry.
    Edit {
        name: &'a str,
        splits: usize,
    },
}

/// The dialog's width: the handoff's 6b and 6c.
pub const WIDTH: gpui::Pixels = px(460.0);

pub type OnFieldClick = Rc<dyn Fn(PayeeField, &mut Window, &mut App)>;
pub type OnOptionClick = select_field::OnOptionClick;

pub struct PayeeDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
    pub on_add_rule: dialog::OnClick,
    pub on_remove_rule: OnRemoveRule,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(
    mode: PayeeDialogMode<'_>,
    form: &PayeeForm,
    options: &PayeeOptions,
    name_error: Option<PayeeError>,
    valid: bool,
    handlers: PayeeDialogHandlers,
    cx: &App,
) -> AnyElement {
    let PayeeDialogHandlers {
        on_field_click,
        on_option_click,
        on_add_rule,
        on_remove_rule,
        on_cancel,
        on_confirm,
    } = handlers;
    let click = |field: PayeeField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let (id, title, placeholder, submit) = match mode {
        PayeeDialogMode::Add => (
            "add-payee",
            crate::msg::desktop_payees_add_title(),
            crate::msg::desktop_payees_rule_placeholder(),
            crate::msg::desktop_payees_add_submit(),
        ),
        PayeeDialogMode::Edit { name, .. } => (
            "edit-payee",
            crate::msg::desktop_payees_edit_title(name),
            crate::msg::desktop_payees_edit_rule_placeholder(),
            crate::msg::desktop_payees_edit_submit(),
        ),
    };
    let element_id = |suffix: &str| SharedString::from(format!("{id}-{suffix}"));
    let (name_id, category_id) = match mode {
        PayeeDialogMode::Add => ("add-payee-name", "add-payee-category"),
        PayeeDialogMode::Edit { .. } => ("edit-payee-name", "edit-payee-category"),
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body([
            div()
                .child(text_field(
                    name_id,
                    label(lib_locale::msg::column_name()),
                    form.name.text(),
                    &crate::msg::desktop_payees_name_placeholder(),
                    form.focused == PayeeField::Name,
                    click(PayeeField::Name),
                    cx,
                ))
                .children(name_error.map(|error| rules_field::error_line(&error, cx)))
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: category_id,
                    label: format!(
                        "{} {}",
                        crate::msg::desktop_payees_field_default_category(),
                        crate::msg::desktop_field_optional()
                    )
                    .into(),
                    options: &options.labels,
                    state: &form.default_category,
                    focused: form.focused == PayeeField::DefaultCategory,
                    accent: false,
                    read_only: None,
                    on_field_click: click(PayeeField::DefaultCategory),
                    on_option_click,
                },
                cx,
            ),
            div()
                .child(rules_field::render(
                    RulesFieldProps {
                        id,
                        rules: &form.rules,
                        input: form.rule_input.text(),
                        placeholder,
                        focused: form.focused == PayeeField::Rule,
                        on_input_click: click(PayeeField::Rule),
                        on_add_click: on_add_rule,
                        on_remove_rule,
                    },
                    cx,
                ))
                .children(
                    form.error
                        .as_ref()
                        .map(|error| rules_field::error_line(error, cx)),
                )
                .into_any_element(),
            callout(mode, cx),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button(element_id("cancel"), on_cancel, cx).into_any_element(),
                dialog::confirm_button(element_id("confirm"), submit, valid, false, on_confirm, cx)
                    .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}

/// Add's dark-bordered note on how rules match; on Edit, while Splits carry the Payee, the handoff's
/// red-bordered usage warning takes its place.
fn callout(mode: PayeeDialogMode<'_>, cx: &App) -> AnyElement {
    let (text, border) = match mode {
        PayeeDialogMode::Edit { splits, .. } if splits > 0 => (
            crate::msg::desktop_payees_edit_usage_callout(
                i64::try_from(splits).unwrap_or(i64::MAX),
            ),
            color::negative(cx),
        ),
        _ => (
            crate::msg::desktop_payees_rules_callout(),
            color::foreground(cx),
        ),
    };
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(border)
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}
