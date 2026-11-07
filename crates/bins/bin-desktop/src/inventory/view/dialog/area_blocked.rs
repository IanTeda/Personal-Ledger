//! The notice a Location's last Area gets in place of a Remove while it holds Items, on the shared
//! `crate::dialog` frame: an info panel and a single **Close** button. Its Presenter closes it on
//! `select` and `submit`, and every other key goes nowhere.

use gpui::{AnyElement, App, div, prelude::*};
use lib_inventory::presenter::{AreaBlockedPresenter, AreaBlockedViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The open notice's clicks.
type Clicks = ModuleDialogClicks<AreaBlockedPresenter>;

/// The `lib-inventory` Area blocked Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for AreaBlockedPresenter {
    type Widgets = ();

    fn kind(_view: &AreaBlockedViewModel) -> &'static str {
        "AreaBlocked"
    }

    fn render(
        view: &AreaBlockedViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &AreaBlockedViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, Vec::new())
    }

    fn hints(view: &AreaBlockedViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &AreaBlockedViewModel, actions: &Clicks, cx: &App) -> AnyElement {
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
