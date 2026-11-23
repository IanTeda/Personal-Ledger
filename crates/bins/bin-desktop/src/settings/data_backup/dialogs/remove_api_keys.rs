//! The Data & backup page's **Remove all API keys** confirm (#1392), on the shared `crate::dialog` frame's
//! destructive variant, an Entity in the Dialog host (`shell::view::dialog_entity`). Nothing to
//! type: `enter` or the button deletes every key, `esc` or Cancel keeps them. Every Dialog command reaches
//! `lib-settings`' `RemoveApiKeysPresenter` as a `DialogInput`; this draws its view model, hands a
//! confirm back to the page and tells the host to close.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::{RemoveApiKeysPresenter, RemoveApiKeysViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};

/// `lib-settings`' [`RemoveApiKeysPresenter`] hosted in a `ModuleDialog`: its confirm lands on the
/// Data & backup page that opened it.
impl DialogView for RemoveApiKeysPresenter {
    type Widgets = ();

    fn kind(_view: &RemoveApiKeysViewModel) -> &'static str {
        "RemoveApiKeys"
    }

    fn render(
        view: &RemoveApiKeysViewModel,
        _widgets: &mut (),
        clicks: &ModuleDialogClicks<Self>,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// Remove all API keys shows no field, so its readback is the kind alone.
    fn readback(_view: &RemoveApiKeysViewModel) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn hints(view: &RemoveApiKeysViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
    view: &RemoveApiKeysViewModel,
    clicks: &ModuleDialogClicks<RemoveApiKeysPresenter>,
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
                dialog::cancel_button("remove-api-keys-cancel", clicks.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-api-keys-confirm",
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
