//! The Delete document confirm on the shared `crate::dialog` frame, destructive: what goes (the
//! Document and its Links), what stays (the file and the records), and that it cannot be undone.
//! What each key means is the Presenter's: `select` and `submit` confirm and every other input is
//! swallowed.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::DeleteDocumentViewModel;

use super::Clicks;
use crate::{dialog, theme::color};

pub(super) fn render(view: &DeleteDocumentViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(view.body.clone())
            .into_any_element(),
        div()
            .text_size(px(13.0))
            .child(dialog::rich_text(view.warning.clone(), None))
            .into_any_element(),
    ];
    fields.extend(view.refusal.clone().map(|text| {
        div()
            .mt(px(6.0))
            .text_size(px(11.5))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_color(color::accent_text(cx))
            .child(text)
            .into_any_element()
    }));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), true, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-document-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-document-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    true,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, true, card, cx)
}
