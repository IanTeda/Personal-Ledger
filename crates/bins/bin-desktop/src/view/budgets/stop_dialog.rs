//! Renders the **Stop budgeting** dialog (`docs/ux/desktop/14-budgets-v2/README.md`'s 9g) on
//! the shared `crate::dialog` chrome at the handoff's 420px. A Stop deletes nothing (a later
//! Onward amount resumes the Category), so there's no typed confirmation and the confirm button
//! keeps the neutral dark fill rather than the destructive one.

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::{
    dialog,
    limit_form::{LimitOptions, StopForm},
    view::accounts::select_field::{self, SelectFieldProps},
};

use super::limit_dialog::{error_line, error_text};

/// The dialog's width: the handoff's 9g.
pub const WIDTH: gpui::Pixels = px(420.0);

pub struct StopDialogProps<'a> {
    pub category: &'a str,
    pub form: &'a StopForm,
    pub options: &'a LimitOptions,
    /// The Category's Bill Plan names, which keep counting as Known Costs.
    pub bill_plans: Vec<String>,
    pub on_field_click: dialog::OnClick,
    pub on_option_click: select_field::OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(props: StopDialogProps<'_>, cx: &App) -> AnyElement {
    let StopDialogProps {
        category,
        form,
        options,
        bill_plans,
        on_field_click,
        on_option_click,
        on_cancel,
        on_confirm,
    } = props;
    let mut fields = vec![
        select_field::render(
            SelectFieldProps {
                id: "budgets-stop-from",
                label: crate::msg::desktop_budgets_stop_field_from().into(),
                options: &options.month_labels,
                state: &form.from,
                focused: true,
                accent: false,
                read_only: None,
                on_field_click,
                on_option_click,
            },
            cx,
        ),
        div()
            .text_size(px(13.0))
            .child(crate::msg::desktop_budgets_stop_body(category))
            .into_any_element(),
    ];
    if !bill_plans.is_empty() {
        fields.push(
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_budgets_stop_body_bills(
                    &bill_plans.join(", "),
                ))
                .into_any_element(),
        );
    }
    if let Some(error) = form.error.as_ref() {
        fields.push(error_line(error_text(error), cx));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_budgets_stop_title(category),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-stop-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "budgets-stop-confirm",
                    crate::msg::desktop_budgets_stop_submit(),
                    true,
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
