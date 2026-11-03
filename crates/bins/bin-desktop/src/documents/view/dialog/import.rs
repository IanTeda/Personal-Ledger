//! **Import files** on the shared `crate::dialog` frame: a multi-line box, one path a line.
//! `select` types a new line, so `submit` imports; why each path of an import that took none
//! failed shows under the box until the paths change.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::ImportViewModel;

use super::Clicks;
use super::document_form::error_line;
use crate::dialog;
use crate::form::dialog_fields::label;
use crate::theme::color;

pub(super) fn render(view: &ImportViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let lines = if view.paths.is_empty() {
        div()
            .text_color(color::faint_text(cx))
            .child(format!("{}\u{2502}", view.placeholder))
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .children(
                view.paths
                    .split('\n')
                    .map(|line| div().min_h(px(16.0)).child(line.to_string())),
            )
            .child(div().text_color(color::accent(cx)).child("\u{2502}"))
            .into_any_element()
    };
    let field = div()
        .child(label(view.label.clone()))
        .child(
            div()
                .id("documents-import-paths")
                .w_full()
                .min_h(px(96.0))
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::accent(cx))
                .text_size(px(12.5))
                .child(lines),
        )
        .children(view.notes.iter().map(|note| error_line(note.clone(), cx)))
        .into_any_element();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            field,
            dialog::info_panel(view.note.clone(), cx).into_any_element(),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("import-documents-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "import-documents-confirm",
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
