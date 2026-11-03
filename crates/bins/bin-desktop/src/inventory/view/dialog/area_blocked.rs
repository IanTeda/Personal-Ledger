//! The notice a Location's last Area gets in place of a Remove while it holds Items, on the shared
//! `crate::dialog` frame: an info panel and a single **Close** button. Its Presenter closes it on
//! `select` and `submit`, and every other key goes nowhere.

use gpui::{AnyElement, App, div, prelude::*};
use lib_inventory::presenter::AreaBlockedViewModel;

use super::Clicks;
use crate::dialog;

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
