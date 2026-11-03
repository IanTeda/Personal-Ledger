//! Accept all strong matches' count-first confirm on the shared `crate::dialog` frame. What each key means is the
//! Presenter's: `select` and `submit` confirm and every other input is swallowed.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::AcceptAllViewModel;

use super::Clicks;
use crate::dialog;

pub(super) fn render(view: &AcceptAllViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            div()
                .text_size(px(13.0))
                .child(view.body.clone())
                .into_any_element(),
            dialog::info_panel(view.note.clone(), cx).into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("accept-all-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "accept-all-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}
