//! The Skip this cycle Dialog (8e) on the shared `crate::dialog` frame at the handoff's 400px.
//! Skipping destroys nothing (Unskip reverses it), so there's no typed-name confirm and the confirm
//! button keeps the neutral styling.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_bills::presenter::{SkipPresenter, SkipViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<SkipPresenter>;

/// The Dialog's width: the handoff's 8e.
const WIDTH: gpui::Pixels = px(400.0);

/// The `lib-bills` Skip Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
impl DialogView for SkipPresenter {
    type Widgets = ();

    fn kind(_view: &SkipViewModel) -> &'static str {
        "Skip"
    }

    fn render(view: &SkipViewModel, _widgets: &mut (), clicks: &Clicks, cx: &App) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &SkipViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, vec![("body", view.body.clone())])
    }

    fn hints(view: &SkipViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &SkipViewModel, actions: &Clicks, cx: &App) -> AnyElement {
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
