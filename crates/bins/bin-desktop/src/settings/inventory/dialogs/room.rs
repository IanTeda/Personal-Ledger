//! The Add, Edit and Remove **Room** dialogs (#495) and the notice the last Room holding Items
//! gets instead of a Remove, on the shared `crate::dialog` chrome.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use super::{count, note};
use crate::{
    dialog,
    form::fields::text_field,
    form::select::SelectState,
    form::select_field::{self, OnOptionClick, SelectFieldProps},
    inventory::NameError,
    settings::inventory::form::RoomForm,
};

pub struct RoomFormHandlers {
    pub on_name_click: dialog::OnClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct RoomFormProps<'a> {
    pub adding: bool,
    pub property: &'a str,
    pub form: &'a RoomForm,
    pub problem: Option<NameError>,
    pub handlers: RoomFormHandlers,
}

/// Add or Edit room: the one Name field, whose error shows once it has been typed in.
pub fn render_room_form(props: RoomFormProps<'_>, cx: &App) -> AnyElement {
    let RoomFormProps {
        adding,
        property,
        form,
        problem,
        handlers,
    } = props;
    let (prefix, title, submit) = if adding {
        (
            "add-room",
            crate::msg::desktop_inventory_room_add_title(),
            crate::msg::desktop_inventory_room_add_submit(),
        )
    } else {
        (
            "edit-room",
            crate::msg::desktop_inventory_room_edit_title(),
            crate::msg::desktop_inventory_room_edit_submit(),
        )
    };
    let shown = problem.filter(|_| form.shows_problem());
    let body = div()
        .child(text_field(
            "room-name",
            crate::msg::desktop_inventory_room_field_name(),
            form.name.text(),
            &crate::msg::desktop_inventory_room_field_name_placeholder(),
            true,
            handlers.on_name_click,
            cx,
        ))
        .child(match shown {
            Some(error) => note(
                match error {
                    NameError::Empty => crate::msg::desktop_inventory_room_error_name_empty(),
                    NameError::Taken => crate::msg::desktop_inventory_room_error_name_taken(),
                },
                true,
                cx,
            )
            .into_any_element(),
            None => note(
                crate::msg::desktop_inventory_room_in_property(property),
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

pub struct RoomRemoveHandlers {
    pub on_field_click: dialog::OnClick,
    pub on_option_click: OnOptionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct RoomRemoveProps<'a> {
    pub name: &'a str,
    pub items: usize,
    /// The Property's other Rooms, which can take the Items.
    pub destinations: &'a [String],
    pub destination: &'a SelectState,
    pub handlers: RoomRemoveHandlers,
}

/// Remove room: a plain confirm while it is empty, else the red dialog with the "Move items to"
/// picker.
pub fn render_room_remove(props: RoomRemoveProps<'_>, cx: &App) -> AnyElement {
    let RoomRemoveProps {
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
                .child(crate::msg::desktop_inventory_room_remove_with_items(
                    name,
                    &crate::msg::desktop_inventory_count_items(count(items)),
                ))
                .into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "room-destination",
                    label: crate::msg::desktop_inventory_room_remove_destination().into(),
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
                .child(crate::msg::desktop_inventory_room_remove_plain(name))
                .into_any_element(),
        ]
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_inventory_room_remove_title(),
            has_items,
            cx,
        ))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-room-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-room-confirm",
                    crate::msg::desktop_inventory_room_remove_submit(),
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

/// The notice for the last Room of a Property while it holds Items: nowhere to move them.
pub fn render_room_blocked(
    name: &str,
    property: &str,
    items: usize,
    on_close: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_inventory_room_blocked_title(),
            false,
            cx,
        ))
        .child(dialog::body([dialog::info_panel(
            crate::msg::desktop_inventory_room_blocked_body(
                name,
                property,
                &crate::msg::desktop_inventory_count_items(count(items)),
            ),
            cx,
        )
        .into_any_element()]))
        .child(dialog::action_row(
            [dialog::confirm_button(
                "room-blocked-close",
                crate::msg::desktop_inventory_room_blocked_close(),
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
