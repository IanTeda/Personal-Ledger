//! The Add, Edit and Remove **Area** dialogs (#495) and the notice the last Area holding Items
//! gets instead of a Remove, on the shared `crate::dialog` frame.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use super::{count, note};
use crate::{
    dialog,
    form::fields::text_field,
    form::select::SelectState,
    form::select_field::{self, OnOptionClick, SelectFieldProps},
    inventory::NameError,
    settings::inventory::form::AreaForm,
};

pub struct AreaFormHandlers {
    pub on_name_click: dialog::OnClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct AreaFormProps<'a> {
    pub adding: bool,
    pub location: &'a str,
    pub form: &'a AreaForm,
    pub problem: Option<NameError>,
    pub handlers: AreaFormHandlers,
}

/// Add or Edit area: the one Name field, whose error shows once it has been typed in.
pub fn render_area_form(props: AreaFormProps<'_>, cx: &App) -> AnyElement {
    let AreaFormProps {
        adding,
        location,
        form,
        problem,
        handlers,
    } = props;
    let (prefix, title, submit) = if adding {
        (
            "add-area",
            crate::msg::desktop_inventory_area_add_title(),
            crate::msg::desktop_inventory_area_add_submit(),
        )
    } else {
        (
            "edit-area",
            crate::msg::desktop_inventory_area_edit_title(),
            crate::msg::desktop_inventory_area_edit_submit(),
        )
    };
    let shown = problem.filter(|_| form.shows_problem());
    let body = div()
        .child(text_field(
            "area-name",
            crate::msg::desktop_inventory_area_field_name(),
            form.name.text(),
            &crate::msg::desktop_inventory_area_field_name_placeholder(),
            true,
            handlers.on_name_click,
            cx,
        ))
        .child(match shown {
            Some(error) => note(
                match error {
                    NameError::Empty => crate::msg::desktop_inventory_area_error_name_empty(),
                    NameError::Taken => crate::msg::desktop_inventory_area_error_name_taken(),
                },
                true,
                cx,
            )
            .into_any_element(),
            None => note(
                crate::msg::desktop_inventory_area_in_location(location),
                false,
                cx,
            )
            .into_any_element(),
        })
        .into_any_element();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(dialog::body([body]))
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

pub struct AreaRemoveHandlers {
    pub on_field_click: dialog::OnClick,
    pub on_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct AreaRemoveProps<'a> {
    pub name: &'a str,
    pub items: usize,
    /// The Location's other Areas, which can take the Items.
    pub destinations: &'a [String],
    pub destination: &'a SelectState,
    pub handlers: AreaRemoveHandlers,
}

/// Remove area: a plain confirm while it is empty, else the red dialog with the "Move items to"
/// picker.
pub fn render_area_remove(props: AreaRemoveProps<'_>, cx: &App) -> AnyElement {
    let AreaRemoveProps {
        name,
        items,
        destinations,
        destination,
        handlers,
    } = props;
    let has_items = items > 0;
    let body: Vec<AnyElement> = if has_items {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_inventory_area_remove_with_items(
                    name,
                    &crate::msg::desktop_inventory_count_items(count(items)),
                ))
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "area-destination",
                    label: crate::msg::desktop_inventory_area_remove_destination().into(),
                    options: destinations,
                    state: destination,
                    focused: true,
                    accent: false,
                    read_only: None,
                    on_field_click: handlers.on_field_click,
                    on_option_click: handlers.on_option_click,
                },
                cx,
            ),
        ]
    } else {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_inventory_area_remove_plain(name))
                .into_any_element(),
        ]
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_inventory_area_remove_title(),
            has_items,
            cx,
        ))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-area-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-area-confirm",
                    crate::msg::desktop_inventory_area_remove_submit(),
                    true,
                    has_items,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, has_items, card, cx)
}

/// The notice for the last Area of a Location while it holds Items: nowhere to move them.
pub fn render_area_blocked(
    name: &str,
    location: &str,
    items: usize,
    on_close: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_inventory_area_blocked_title(),
            false,
            cx,
        ))
        .child(dialog::body([dialog::info_panel(
            crate::msg::desktop_inventory_area_blocked_body(
                name,
                location,
                &crate::msg::desktop_inventory_count_items(count(items)),
            ),
            cx,
        )
        .into_any_element()]))
        .child(dialog::action_row(
            [dialog::confirm_button(
                "area-blocked-close",
                crate::msg::desktop_inventory_area_blocked_close(),
                true,
                false,
                on_close,
                cx,
            )
            .into_any_element()],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, false, card, cx)
}
