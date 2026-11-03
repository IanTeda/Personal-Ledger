//! The Add and Edit location Dialogs on the shared `crate::dialog` frame. Add and Edit share one
//! renderer: the Name and the Unit (a picker on Add, shown disabled on Edit), and nothing else,
//! since a Location's address and cover are edited from the Inventory surface (16q).
//!
//! The Presenter reads every Dialog command and typed key itself; the View draws its view model
//! and turns each click into an event.

use gpui::{AnyElement, App, SharedString, div, prelude::*};
use lib_inventory::presenter::{
    LocationField, LocationFormEvent, LocationFormPresenter, LocationFormViewModel,
};

use super::{Clicks, InventoryDialog, View, note};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::fields::text_field;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::HostRouter;

fn handle(presenter: &mut LocationFormPresenter, event: LocationFormEvent) {
    presenter.handle(event, &mut HostRouter::default());
}

fn form(dialog: &mut InventoryDialog) -> Option<&mut LocationFormPresenter> {
    match &mut dialog.view {
        View::Location(presenter, _) => Some(presenter),
        _ => None,
    }
}

/// A click on the Unit field focuses it and toggles its list.
fn click_unit(dialog: &mut InventoryDialog) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LocationFormEvent::ToggleUnits);
    }
}

/// A click on row `index` of the open list.
fn click_option(dialog: &mut InventoryDialog, index: usize) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LocationFormEvent::Focus(LocationField::Unit));
        if let Some(choice) = presenter.view_model().unit.select.options.get(index) {
            handle(presenter, LocationFormEvent::Unit(choice.value));
        }
    }
}

/// A click on a field focuses it, closing the list.
fn focus(dialog: &mut InventoryDialog, field: LocationField) {
    if let Some(presenter) = form(dialog) {
        handle(presenter, LocationFormEvent::Focus(field));
    }
}

pub(super) fn render(
    view: &LocationFormViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    // `text_field` and the buttons take a `'static` id, so these are spelt out.
    let prefix = if view.editing {
        "edit-location"
    } else {
        "add-location"
    };
    let options = labels(&view.unit.select);
    let state = drawn(&view.unit.select, view.unit_list, scroll);
    let name_error = view.name_error.clone().map(|text| note(text, true, cx));
    let fields: Vec<AnyElement> = vec![
        div()
            .child(text_field(
                "location-name",
                view.name.label.clone(),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                actions.click(|dialog| focus(dialog, LocationField::Name)),
                cx,
            ))
            .children(name_error)
            .into_any_element(),
        div()
            .child(select_field::render(
                SelectFieldProps {
                    id: "location-unit".into(),
                    label: view.unit.select.label.clone().into(),
                    options: &options,
                    state: &state,
                    focused: view.unit.select.focused,
                    accent: false,
                    read_only: view.unit.fixed.clone().map(SharedString::from),
                    on_field_click: actions.click(click_unit),
                    on_option_click: actions.option(click_option),
                },
                cx,
            ))
            .child(note(view.unit.hint.clone(), false, cx))
            .children(view.unit.error.clone().map(|text| note(text, true, cx)))
            .into_any_element(),
    ];
    let mut fields = fields;
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
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
