//! The Remove location Dialog on the shared `crate::dialog` frame: a plain confirm for an empty
//! Location and otherwise the red dialog that quotes what goes with it and asks for the name typed
//! back, which holds every typed key.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_inventory::presenter::{
    RemoveLocationEvent, RemoveLocationPresenter, RemoveLocationViewModel,
};
use lib_shell::Hint;

use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::fields::text_field;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<RemoveLocationPresenter>;

/// The `lib-inventory` Remove location Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for RemoveLocationPresenter {
    type Widgets = ();

    fn kind(_view: &RemoveLocationViewModel) -> &'static str {
        "RemoveLocation"
    }

    fn render(
        view: &RemoveLocationViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    /// The name typed back, when the Location holds anything.
    fn readback(view: &RemoveLocationViewModel) -> Vec<(&'static str, String)> {
        let typed = view
            .confirm
            .as_ref()
            .map(|field| field.text.clone())
            .unwrap_or_default();
        framed(&view.frame, vec![("confirm", typed)])
    }

    fn hints(view: &RemoveLocationViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &RemoveLocationViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut body = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    body.extend(view.confirm.as_ref().map(|field| {
        text_field(
            "remove-location-confirm-input",
            field.label.clone(),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.send_all(Vec::<RemoveLocationEvent>::new()),
            cx,
        )
        .into_any_element()
    }));
    body.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-location-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-location-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, destructive, card, cx)
}
