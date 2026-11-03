//! The Remove area Dialog (#487) on the shared `crate::dialog` frame: a plain confirm while the
//! Area is empty, else the red dialog with the "Move items to" select, pre-selected on the Area
//! above (below when first).
//!
//! The Presenter reads every Dialog command and typed key itself; the View draws its view model
//! and turns each click into an event.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_inventory::presenter::{RemoveAreaEvent, RemoveAreaPresenter, RemoveAreaViewModel};

use super::{Clicks, InventoryDialog, View};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::HostRouter;

fn handle(presenter: &mut RemoveAreaPresenter, event: RemoveAreaEvent) {
    presenter.handle(event, &mut HostRouter::default());
}

fn remove(dialog: &mut InventoryDialog) -> Option<&mut RemoveAreaPresenter> {
    match &mut dialog.view {
        View::RemoveArea(presenter, _) => Some(presenter),
        _ => None,
    }
}

/// A click on the destination field toggles its list.
fn click_field(dialog: &mut InventoryDialog) {
    if let Some(presenter) = remove(dialog) {
        handle(presenter, RemoveAreaEvent::ToggleDestinations);
    }
}

/// A click on row `index` of the open list.
fn click_option(dialog: &mut InventoryDialog, index: usize) {
    if let Some(presenter) = remove(dialog)
        && let Some(destination) = presenter.view_model().destination
        && let Some(choice) = destination.options.get(index)
    {
        handle(presenter, RemoveAreaEvent::Destination(choice.value));
    }
}

pub(super) fn render(
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
                on_field_click: actions.click(click_field),
                on_option_click: actions.option(click_option),
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
