//! The Add and Edit location Dialogs on the shared `crate::dialog` frame. Add and Edit share one
//! renderer: the Name and the Unit (a picker on Add, shown disabled on Edit), and nothing else,
//! since a Location's address and cover are edited from the Inventory surface (16q).
//!
//! The Unit select takes the arrows and `Space` while it has focus, `Enter` commits an open list,
//! else confirms, and typing goes nowhere on it rather than reaching the Shell.

use gpui::{AnyElement, App, SharedString, div, prelude::*};
use lib_inventory::presenter::{
    LocationField, LocationFormEvent, LocationFormPresenter, LocationFormViewModel,
};

use super::{Clicks, InventoryDialog, View, note};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::fields::text_field;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select, typed};
use crate::inventory::router::DialogRouter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

fn handle(presenter: &mut LocationFormPresenter, event: LocationFormEvent) {
    presenter.handle(event, &mut DialogRouter::default());
}

pub(super) fn key(
    presenter: &mut LocationFormPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        // Leaving the Unit closes its list, as a click elsewhere would.
        DialogKey::Tab | DialogKey::BackTab => {
            select.cancel();
            let event = if key == DialogKey::Tab {
                LocationFormEvent::NextField
            } else {
                LocationFormEvent::PrevField
            };
            handle(presenter, event);
            return DialogOutcome::Handled;
        }
        DialogKey::Enter if !(view.focused == LocationField::Unit && select.is_open()) => {
            return enter(view.frame.can_confirm);
        }
        _ => {}
    }
    match view.focused {
        LocationField::Name => match typed(&view.name.text, key) {
            Some(text) => {
                handle(presenter, LocationFormEvent::Name(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        LocationField::Unit => match key {
            DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
                let (_, value) = select_key(select, &view.unit.select, key);
                if let Some(id) = value {
                    handle(presenter, LocationFormEvent::Unit(id));
                }
                DialogOutcome::Handled
            }
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
    }
}

fn form(dialog: &mut InventoryDialog) -> Option<(&mut LocationFormPresenter, &mut SelectState)> {
    match &mut dialog.view {
        View::Location(presenter, select) => Some((presenter, select)),
        _ => None,
    }
}

/// A click on the Unit field focuses it and toggles its list.
fn click_unit(dialog: &mut InventoryDialog) {
    if let Some((presenter, select)) = form(dialog) {
        handle(presenter, LocationFormEvent::Focus(LocationField::Unit));
        toggle_select(select, &presenter.view_model().unit.select);
    }
}

fn click_option(dialog: &mut InventoryDialog, index: usize) {
    if let Some((presenter, select)) = form(dialog) {
        handle(presenter, LocationFormEvent::Focus(LocationField::Unit));
        if let Some(id) = choose_option(select, &presenter.view_model().unit.select, index) {
            handle(presenter, LocationFormEvent::Unit(id));
        }
    }
}

fn focus(dialog: &mut InventoryDialog, field: LocationField) {
    if let Some((presenter, select)) = form(dialog) {
        select.cancel();
        handle(presenter, LocationFormEvent::Focus(field));
    }
}

pub(super) fn render(
    view: &LocationFormViewModel,
    select: &SelectState,
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
                    id: "location-unit",
                    label: view.unit.select.label.clone().into(),
                    options: &options,
                    state: &synced(select, &view.unit.select),
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
