//! The Add, Edit and Remove **Location** dialogs on the shared `crate::dialog` chrome. Add and Edit
//! share one renderer: the Name and the Unit (a picker on Add, shown disabled on Edit), and nothing
//! else, since a Location's address and cover are edited from the Inventory surface (16q). Remove
//! is a plain confirm for an empty Location and otherwise the red dialog that asks for the name
//! typed. The form holds the live state, and `Shell`'s Dialog host routes every key and click to
//! it; this module only draws.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::{count, note};
use crate::settings::inventory::form::{LocationField, LocationForm, Problem};
use crate::{
    dialog,
    form::fields::text_field,
    form::select_field::{self, OnOptionClick, SelectFieldProps},
};

pub type OnFieldClick = Rc<dyn Fn(LocationField, &mut Window, &mut App)>;

pub struct FormHandlers {
    pub on_field_click: OnFieldClick,
    pub on_unit_click: dialog::OnClick,
    pub on_unit_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct FormProps<'a> {
    pub form: &'a LocationForm,
    /// Add shows the picker, Edit this read-only text instead.
    pub fixed_unit: Option<String>,
    /// The Unit options (labels), fiat only.
    pub unit_options: &'a [String],
    pub problem: Option<Problem>,
    pub handlers: FormHandlers,
}

fn problem_text(problem: Problem) -> String {
    match problem {
        Problem::NameEmpty => crate::msg::desktop_inventory_error_name_empty(),
        Problem::NameTaken => crate::msg::desktop_inventory_error_name_taken(),
        Problem::UnitMissing => crate::msg::desktop_inventory_error_unit_missing(),
    }
}

pub fn render_form(props: FormProps<'_>, cx: &App) -> AnyElement {
    let FormProps {
        form,
        fixed_unit,
        unit_options,
        problem,
        handlers,
    } = props;
    let (prefix, title, submit) = if form.adding {
        (
            "add-location",
            crate::msg::desktop_inventory_dialog_add_title(),
            crate::msg::desktop_inventory_dialog_add_submit(),
        )
    } else {
        (
            "edit-location",
            crate::msg::desktop_inventory_dialog_edit_title(),
            crate::msg::desktop_inventory_dialog_save(),
        )
    };
    // The one problem shown is the first, and only once it has been earned.
    let shown = problem.filter(|problem| form.shows(*problem));
    let error_for = |field: LocationField, cx: &App| {
        shown
            .filter(|problem| problem.field() == field)
            .map(|problem| note(problem_text(problem), true, cx).into_any_element())
    };
    let on_name_click: dialog::OnClick = {
        let on_click = handlers.on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(LocationField::Name, window, cx))
    };
    let fields: Vec<AnyElement> = vec![
        div()
            .child(text_field(
                "location-name",
                crate::msg::desktop_inventory_field_name(),
                form.name.text(),
                &crate::msg::desktop_inventory_field_name_placeholder(),
                form.focused == LocationField::Name,
                on_name_click,
                cx,
            ))
            .children(error_for(LocationField::Name, cx))
            .into_any_element(),
        div()
            .child(select_field::render(
                SelectFieldProps {
                    id: "location-unit",
                    label: crate::msg::desktop_inventory_field_unit().into(),
                    options: unit_options,
                    state: &form.unit,
                    focused: form.focused == LocationField::Unit,
                    accent: false,
                    read_only: fixed_unit.map(SharedString::from),
                    on_field_click: handlers.on_unit_click.clone(),
                    on_option_click: handlers.on_unit_option_click.clone(),
                },
                cx,
            ))
            .child(note(
                crate::msg::desktop_inventory_field_unit_hint(),
                false,
                cx,
            ))
            .children(error_for(LocationField::Unit, cx))
            .into_any_element(),
    ];

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    handlers.on_cancel,
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
                    submit,
                    problem.is_none(),
                    false,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

pub struct RemoveHandlers {
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

/// What Remove says about the Location: its name and what goes with it.
pub struct RemoveProps<'a> {
    pub name: &'a str,
    pub areas: usize,
    pub items: usize,
    /// The Documents that lose a Link to one of the Items.
    pub documents: usize,
    /// The name typed so far, asked for only when there is something to lose.
    pub typed: &'a str,
    pub handlers: RemoveHandlers,
}

pub fn render_remove(props: RemoveProps<'_>, cx: &App) -> AnyElement {
    let RemoveProps {
        name,
        areas,
        items,
        documents,
        typed,
        handlers,
    } = props;
    let holds_something = areas > 0 || items > 0;
    let body: Vec<AnyElement> = if holds_something {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_inventory_remove_warning(
                    name,
                    &crate::msg::desktop_inventory_count_areas(count(areas)),
                    &crate::msg::desktop_inventory_count_items(count(items)),
                    &crate::msg::desktop_inventory_count_documents(count(documents)),
                ))
                .into_any_element(),
            text_field(
                "remove-location-confirm-input",
                crate::msg::desktop_inventory_remove_confirm_label(name),
                typed,
                name,
                true,
                Rc::new(|_window: &mut Window, _cx: &mut App| {}),
                cx,
            )
            .into_any_element(),
        ]
    } else {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_inventory_remove_plain(name))
                .into_any_element(),
        ]
    };
    let enabled = !holds_something || typed == name;
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_inventory_dialog_remove_title(),
            holds_something,
            cx,
        ))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-location-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-location-confirm",
                    crate::msg::desktop_inventory_dialog_remove_submit(),
                    enabled,
                    holds_something,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, holds_something, card, cx)
}
