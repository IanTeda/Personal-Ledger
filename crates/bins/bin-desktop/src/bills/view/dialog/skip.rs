//! The Skip this cycle Dialog (8e) on the shared `crate::dialog` chrome at the handoff's 400px.
//! Skipping destroys nothing (Unskip reverses it), so there's no typed-name confirm and the confirm
//! button keeps the neutral styling.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_bills::presenter::SkipViewModel;

use crate::chrome::dialog_host::DialogActions;
use crate::dialog;

/// The Dialog's width: the handoff's 8e.
const WIDTH: gpui::Pixels = px(400.0);

pub(super) fn render(view: &SkipViewModel, actions: &DialogActions, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(vec![
            div()
                .text_size(px(13.0))
                .child(view.body.clone())
                .into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("skip-bill-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "skip-bill-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    view.frame.destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}
