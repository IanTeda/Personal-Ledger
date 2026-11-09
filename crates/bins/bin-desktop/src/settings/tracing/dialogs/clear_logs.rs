//! The Tracing page's **Clear logs** confirm (issue #502), on the shared `crate::dialog` frame's
//! destructive variant, an Entity in the Dialog host (`shell::view::dialog_entity`). Nothing to
//! type: `enter` or the button clears, `esc` or Cancel keeps. Every Dialog command reaches
//! `lib-settings`' `ClearLogsPresenter` as a `DialogInput`; this draws its view model, hands a
//! confirm back to the page and tells the host to close.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::{ClearLogsPresenter, ClearLogsViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};

/// `lib-settings`' [`ClearLogsPresenter`] hosted in a `ModuleDialog`: its confirm lands on the
/// Tracing page that opened it.
impl DialogView for ClearLogsPresenter {
    type Widgets = ();

    fn kind(_view: &ClearLogsViewModel) -> &'static str {
        "ClearLogs"
    }

    fn render(
        view: &ClearLogsViewModel,
        _widgets: &mut (),
        clicks: &ModuleDialogClicks<Self>,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// Clear logs shows no field, so its readback is the kind alone.
    fn readback(_view: &ClearLogsViewModel) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn hints(view: &ClearLogsViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
    view: &ClearLogsViewModel,
    clicks: &ModuleDialogClicks<ClearLogsPresenter>,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.title.clone(), true, cx))
        .child(dialog::body([div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element()]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("clear-logs-cancel", clicks.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "clear-logs-confirm",
                    view.confirm.clone(),
                    true,
                    true,
                    clicks.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, true, card, cx)
}
