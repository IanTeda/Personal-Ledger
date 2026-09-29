//! Renders the **Skip this cycle** dialog (`docs/ux/desktop/Bills/README.md`'s 8e) on the shared
//! `crate::dialog` chrome at the handoff's 400px. Skipping resolves one Schedule entry and
//! destroys nothing (Unskip reverses it), so there's no typed-name confirm and the confirm button
//! keeps the neutral styling rather than the destructive one. A One-shot Plan's paragraph says
//! the bill is cancelled instead, since it has no next cycle (#368).

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::dialog;

/// The dialog's width: the handoff's 8e.
pub const WIDTH: gpui::Pixels = px(400.0);

pub struct SkipDialogProps<'a> {
    pub plan_name: &'a str,
    /// The entry's due date, already formatted.
    pub due: String,
    pub one_shot: bool,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(props: SkipDialogProps<'_>, cx: &App) -> AnyElement {
    let SkipDialogProps {
        plan_name,
        due,
        one_shot,
        on_cancel,
        on_confirm,
    } = props;
    let paragraph = if one_shot {
        crate::msg::desktop_bills_skip_body_one_shot(&due, plan_name)
    } else {
        crate::msg::desktop_bills_skip_body(&due, plan_name)
    };

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_bills_skip_title(plan_name),
            false,
            cx,
        ))
        .child(dialog::body(vec![
            div()
                .text_size(px(13.0))
                .child(paragraph)
                .into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("skip-bill-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "skip-bill-confirm",
                    crate::msg::desktop_bills_skip_submit(),
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
