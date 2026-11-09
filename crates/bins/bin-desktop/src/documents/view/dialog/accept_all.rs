//! Accept all strong matches' count-first confirm on the shared `crate::dialog` frame. What each key means is the
//! Presenter's: `select` and `submit` confirm and every other input is swallowed.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_documents::presenter::{AcceptAllPresenter, AcceptAllViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

type Clicks = ModuleDialogClicks<AcceptAllPresenter>;

/// The `lib-documents` Accept all Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for AcceptAllPresenter {
    type Widgets = ();

    fn kind(_view: &AcceptAllViewModel) -> &'static str {
        "accept-all"
    }

    fn render(
        view: &AcceptAllViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &AcceptAllViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, vec![("body", view.body.clone())])
    }

    fn hints(view: &AcceptAllViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &AcceptAllViewModel, actions: &Clicks, cx: &App) -> AnyElement {
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
