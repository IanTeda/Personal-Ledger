//! The Tracing page's **Clear logs** confirm (issue #502), on the shared `crate::dialog` chrome's
//! destructive variant. Nothing to type: `enter` or the button clears, `esc` or Cancel keeps.

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::dialog;

pub fn render(on_cancel: dialog::OnClick, on_confirm: dialog::OnClick, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_tracing_clear_title(),
            true,
            cx,
        ))
        .child(dialog::body([div()
            .text_size(px(13.0))
            .child(crate::msg::desktop_settings_tracing_clear_warning())
            .into_any_element()]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("clear-logs-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "clear-logs-confirm",
                    crate::msg::desktop_settings_tracing_clear(),
                    true,
                    true,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, true, card, cx)
}
