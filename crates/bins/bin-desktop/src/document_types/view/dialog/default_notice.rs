//! The Other notice (`dialog_host`'s old `DefaultNotice`): why the Default Document Type has no
//! remove action, on the shared `crate::dialog` frame. Its one button closes it; nothing is
//! written.

use gpui::{AnyElement, App, div, prelude::*};
use lib_documents::presenter::DefaultNoticeViewModel;

use super::Clicks;
use crate::dialog;
use crate::form::viper::enter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

/// `Enter` closes the notice; `Tab` has no field to move to and is swallowed.
pub(super) fn key(key: DialogKey) -> DialogOutcome {
    match key {
        DialogKey::Enter | DialogKey::CtrlEnter => enter(true),
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => DialogOutcome::Ignored,
    }
}

pub(super) fn render(view: &DefaultNoticeViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            dialog::info_panel(view.text.clone(), cx).into_any_element()
        ]))
        .child(dialog::action_row(
            [dialog::confirm_button(
                "document-type-notice-close",
                view.frame.submit.clone(),
                view.frame.can_confirm,
                false,
                actions.confirm.clone(),
                cx,
            )
            .into_any_element()],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// The notice's text, for a test.
pub(super) fn readback(view: &DefaultNoticeViewModel) -> Vec<(&'static str, String)> {
    vec![("text", view.text.clone())]
}
