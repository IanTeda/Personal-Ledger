//! The notice a Location's last Area gets in place of a Remove while it holds Items, on the shared
//! `crate::dialog` frame: an info panel and a single **Close** button. Every key but `Enter` and
//! `Esc` goes nowhere.

use gpui::{AnyElement, App, div, prelude::*};
use lib_inventory::presenter::AreaBlockedViewModel;

use super::Clicks;
use crate::dialog;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

pub(super) fn key(key: DialogKey) -> DialogOutcome {
    match key {
        DialogKey::Enter => DialogOutcome::Confirm,
        DialogKey::Char(_) | DialogKey::Backspace | DialogKey::Tab | DialogKey::BackTab => {
            DialogOutcome::Handled
        }
        _ => DialogOutcome::Ignored,
    }
}

pub(super) fn render(view: &AreaBlockedViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            dialog::info_panel(view.body.clone(), cx).into_any_element()
        ]))
        .child(dialog::action_row(
            [dialog::confirm_button(
                "area-blocked-close",
                view.frame.submit.clone(),
                true,
                false,
                actions.confirm.clone(),
                cx,
            )
            .into_any_element()],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}
