//! The Remove area Dialog (#487) on the shared `crate::dialog` frame: a plain confirm while the
//! Area is empty, else the red dialog with the "Move items to" select, pre-selected on the Area
//! above (below when first).
//!
//! The Presenter reads every Dialog command and typed key itself; the View draws its view model
//! and turns each click into an event.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_inventory::presenter::{RemoveAreaEvent, RemoveAreaPresenter, RemoveAreaViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<RemoveAreaPresenter>;

/// The `lib-inventory` Remove area Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). Its widget state is the destination list's top row.
impl DialogView for RemoveAreaPresenter {
    type Widgets = usize;

    fn kind(_view: &RemoveAreaViewModel) -> &'static str {
        "RemoveArea"
    }

    fn render(
        view: &RemoveAreaViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    fn readback(view: &RemoveAreaViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, Vec::new())
    }

    fn hints(view: &RemoveAreaViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
    view: &RemoveAreaViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut body = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    body.extend(view.destination.as_ref().map(|destination| {
        let options = labels(destination);
        let values: Vec<_> = destination
            .options
            .iter()
            .map(|choice| choice.value)
            .collect();
        let state = drawn(destination, view.destination_list, scroll);
        select_field::render(
            SelectFieldProps {
                id: "area-destination".into(),
                label: destination.label.clone().into(),
                options: &options,
                state: &state,
                focused: destination.focused,
                accent: false,
                read_only: None,
                on_field_click: actions.send(RemoveAreaEvent::ToggleDestinations),
                on_option_click: actions.option(move |row| {
                    values
                        .get(row)
                        .map(|value| RemoveAreaEvent::Destination(*value))
                        .into_iter()
                        .collect()
                }),
            },
            cx,
        )
    }));
    body.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-area-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-area-confirm",
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
