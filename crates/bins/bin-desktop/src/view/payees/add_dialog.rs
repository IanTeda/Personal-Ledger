//! Renders the **Add payee** dialog (`docs/ux/desktop/Payees/README.md`'s 6b) on the shared
//! `crate::dialog` chrome: Name, Default category (the shared `select_field` dropdown, "none"
//! then the leaf Categories), the shared Match rules field, and the dark-bordered callout.
//! `Shell` owns the live form (`payees::PayeeForm`) and every keystroke while it is open.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use super::rules_field::{self, OnRemoveRule, RulesFieldProps};
use crate::{
    dialog,
    payees::{PayeeError, PayeeField, PayeeForm, PayeeOptions},
    theme::color,
    view::accounts::{
        add_dialog::{label, text_field},
        select_field::{self, SelectFieldProps},
    },
};

/// The dialog's width: the handoff's 6b.
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
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_payees_add_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            div()
                .child(text_field(
                    "add-payee-name",
                    label(lib_locale::msg::column_name()),
                    &form.name,
                    &crate::msg::desktop_payees_name_placeholder(),
                    form.focused == PayeeField::Name,
                    click(PayeeField::Name),
                    cx,
                ))
                .children(name_error.map(|error| rules_field::error_line(&error, cx)))
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "add-payee-category",
                    label: format!(
                        "{} {}",
                        crate::msg::desktop_payees_field_default_category(),
                        crate::msg::desktop_field_optional()
                    )
                    .into(),
                    options: &options.labels,
                    state: &form.default_category,
                    focused: form.focused == PayeeField::DefaultCategory,
                    read_only: None,
                    on_field_click: click(PayeeField::DefaultCategory),
                    on_option_click,
                },
                cx,
            ),
            div()
                .child(rules_field::render(
                    RulesFieldProps {
                        id: "add-payee",
                        rules: &form.rules,
                        input: &form.rule_input,
                        placeholder: crate::msg::desktop_payees_rule_placeholder(),
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
            div()
                .p(px(10.0))
                .bg(color::chrome(cx))
                .border_l(px(2.0))
                .border_color(color::foreground(cx))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_payees_rules_callout())
                .into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-payee-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "add-payee-confirm",
                    crate::msg::desktop_payees_add_submit(),
                    valid,
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
