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
use lib_shell::Hint;

use super::note;
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::fields::text_field;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The open form's clicks.
type Clicks = ModuleDialogClicks<LocationFormPresenter>;

/// The `lib-inventory` Location form Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). Its widget state is the Unit list's top row.
impl DialogView for LocationFormPresenter {
    type Widgets = usize;

    fn kind(view: &LocationFormViewModel) -> &'static str {
        if view.editing {
            "EditLocation"
        } else {
            "AddLocation"
        }
    }

    fn render(
        view: &LocationFormViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    fn readback(view: &LocationFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, vec![("name", view.name.text.clone())])
    }

    fn hints(view: &LocationFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(
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
    let units: Vec<_> = view
        .unit
        .select
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
    let name_error = view.name_error.clone().map(|text| note(text, true, cx));
    let fields: Vec<AnyElement> = vec![
        div()
            .child(text_field(
                "location-name",
                view.name.label.clone(),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                actions.send(LocationFormEvent::Focus(LocationField::Name)),
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
                    on_field_click: actions.send(LocationFormEvent::ToggleUnits),
                    on_option_click: actions.option(move |row| {
                        // A row click focuses the Unit field, then picks.
                        let mut events = vec![LocationFormEvent::Focus(LocationField::Unit)];
                        events.extend(units.get(row).map(|unit| LocationFormEvent::Unit(*unit)));
                        events
                    }),
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
