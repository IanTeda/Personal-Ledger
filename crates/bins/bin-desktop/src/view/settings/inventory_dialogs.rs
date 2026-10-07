//! The Add, Edit and Remove **Property** dialogs (#490's variant B) on the shared `crate::dialog`
//! chrome. Add and Edit share one renderer: Name, Address, the Unit (a picker on Add, shown
//! disabled on Edit) and an always-visible cover section whose empty Insurer means no cover.
//! Remove is a plain confirm for an empty Property and otherwise the red dialog that asks for the
//! name typed. `Shell` owns the live state and every key; this module only draws.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::add_unit_dialog::text_field;
use crate::{
    dialog,
    inventory::NameError,
    inventory::form::{Problem, PropertyField, PropertyForm, RoomForm},
    select::SelectState,
    theme::color,
    view::accounts::select_field::{self, OnOptionClick, SelectFieldProps},
};

pub type OnFieldClick = Rc<dyn Fn(PropertyField, &mut Window, &mut App)>;
pub type OnSuggestionClick = Rc<dyn Fn(String, &mut Window, &mut App)>;

pub struct FormHandlers {
    pub on_field_click: OnFieldClick,
    pub on_unit_click: dialog::OnClick,
    pub on_unit_option_click: OnOptionClick,
    pub on_suggestion_click: OnSuggestionClick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub struct FormProps<'a> {
    pub form: &'a PropertyForm,
    /// Add shows the picker, Edit this read-only text instead.
    pub fixed_unit: Option<String>,
    /// The Unit options (labels), fiat only.
    pub unit_options: &'a [String],
    pub suggestions: &'a [String],
    pub problem: Option<Problem>,
    pub valid: bool,
    pub handlers: FormHandlers,
}

fn problem_text(problem: &Problem) -> String {
    match problem {
        Problem::NameEmpty => crate::msg::desktop_inventory_error_name_empty(),
        Problem::NameTaken => crate::msg::desktop_inventory_error_name_taken(),
        Problem::RenewsOn(message) => message.clone(),
        Problem::Amount(_) => crate::msg::desktop_inventory_error_amount(),
        Problem::SumMissing => crate::msg::desktop_inventory_error_sum_missing(),
        Problem::SumNotPositive => crate::msg::desktop_inventory_error_sum_not_positive(),
        Problem::LimitNotPositive => crate::msg::desktop_inventory_error_limit_not_positive(),
        Problem::LimitAboveSum => crate::msg::desktop_inventory_error_limit_above_sum(),
        Problem::UnitMissing => crate::msg::desktop_inventory_error_unit_missing(),
    }
}

fn note(text: String, error: bool, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(if error {
            color::accent_text(cx)
        } else {
            color::muted(cx)
        })
        .child(text)
}

pub fn render_form(props: FormProps<'_>, cx: &App) -> AnyElement {
    let FormProps {
        form,
        fixed_unit,
        unit_options,
        suggestions,
        problem,
        valid,
        handlers,
    } = props;
    let (prefix, title, submit) = if form.adding {
        (
            "add-property",
            crate::msg::desktop_inventory_dialog_add_title(),
            crate::msg::desktop_inventory_dialog_add_submit(),
        )
    } else {
        (
            "edit-property",
            crate::msg::desktop_inventory_dialog_edit_title(),
            crate::msg::desktop_inventory_dialog_save(),
        )
    };
    let click = |field: PropertyField| -> dialog::OnClick {
        let on_click = handlers.on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_click(field, window, cx))
    };
    // The one problem shown is the first, and only once its field has been typed in.
    let shown = problem.filter(|problem| form.shows(problem));
    let error_for = |field: PropertyField, cx: &App| {
        shown
            .as_ref()
            .filter(|problem| problem.field() == field)
            .map(|problem| note(problem_text(problem), true, cx).into_any_element())
    };
    let field = |id: &'static str,
                 field: PropertyField,
                 label: String,
                 placeholder: String,
                 cx: &App|
     -> AnyElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .child(text_field(
                id,
                label,
                form.text(field),
                &placeholder,
                form.focused == field,
                click(field),
                cx,
            ))
            .children(error_for(field, cx))
            .into_any_element()
    };

    let mut fields: Vec<AnyElement> = vec![
        field(
            "property-name",
            PropertyField::Name,
            crate::msg::desktop_inventory_field_name(),
            crate::msg::desktop_inventory_field_name_placeholder(),
            cx,
        ),
        field(
            "property-address",
            PropertyField::Address,
            crate::msg::desktop_inventory_field_address(),
            crate::msg::desktop_inventory_field_address_placeholder(),
            cx,
        ),
    ];
    fields.push(
        div()
            .child(select_field::render(
                SelectFieldProps {
                    id: "property-unit",
                    label: crate::msg::desktop_inventory_field_unit().into(),
                    options: unit_options,
                    state: &form.unit,
                    focused: form.focused == PropertyField::Unit,
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
            .children(error_for(PropertyField::Unit, cx))
            .into_any_element(),
    );
    fields.push(
        div()
            .pt(px(4.0))
            .border_t(px(1.0))
            .border_color(color::hairline(cx))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(10.0))
            .text_color(color::faint_text(cx))
            .child(lib_locale::format::upper(
                &crate::msg::desktop_inventory_section_cover(),
            ))
            .into_any_element(),
    );
    fields.push(
        div()
            .child(field(
                "property-insurer",
                PropertyField::Insurer,
                crate::msg::desktop_inventory_field_insurer(),
                String::new(),
                cx,
            ))
            .child(suggestion_chips(
                suggestions,
                form.focused == PropertyField::Insurer,
                handlers.on_suggestion_click.clone(),
                cx,
            ))
            .child(note(
                crate::msg::desktop_inventory_field_insurer_hint(),
                false,
                cx,
            ))
            .into_any_element(),
    );
    fields.push(
        div()
            .flex()
            .gap(px(12.0))
            .child(field(
                "property-policy-no",
                PropertyField::PolicyNo,
                crate::msg::desktop_inventory_field_policy_no(),
                String::new(),
                cx,
            ))
            .child(field(
                "property-renews-on",
                PropertyField::RenewsOn,
                crate::msg::desktop_inventory_field_renews_on(),
                crate::msg::desktop_inventory_field_renews_on_placeholder(),
                cx,
            ))
            .into_any_element(),
    );
    fields.push(
        div()
            .flex()
            .gap(px(12.0))
            .child(field(
                "property-sum-insured",
                PropertyField::SumInsured,
                crate::msg::desktop_inventory_field_sum_insured(),
                crate::msg::desktop_inventory_field_amount_placeholder(),
                cx,
            ))
            .child(field(
                "property-item-limit",
                PropertyField::ItemLimit,
                crate::msg::desktop_inventory_field_item_limit(),
                crate::msg::desktop_inventory_field_amount_placeholder(),
                cx,
            ))
            .into_any_element(),
    );

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
                    valid,
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

/// The insurers already in use that match what is typed: the Insurer's datalist. Clicking one
/// fills the field.
fn suggestion_chips(
    suggestions: &[String],
    focused: bool,
    on_click: OnSuggestionClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .when(focused && !suggestions.is_empty(), |this| this.mt(px(6.0)))
        .children(
            suggestions
                .iter()
                .enumerate()
                .filter(|_| focused)
                .map(|(index, insurer)| {
                    let id = SharedString::from(format!("property-insurer-suggestion-{index}"));
                    let on_click = on_click.clone();
                    let value = insurer.clone();
                    div()
                        .debug_selector({
                            let id = id.clone();
                            move || id.to_string()
                        })
                        .id(id)
                        .cursor_pointer()
                        .py(px(3.0))
                        .px(px(8.0))
                        .border_1()
                        .border_color(color::border(cx))
                        .text_size(px(11.5))
                        .on_click(move |_event, window, cx| on_click(value.clone(), window, cx))
                        .child(insurer.clone())
                }),
        )
}

pub struct RemoveHandlers {
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

/// What Remove says about the Property: its name and what goes with it.
pub struct RemoveProps<'a> {
    pub name: &'a str,
    pub rooms: usize,
    pub items: usize,
    /// The Documents that lose a Link to one of the Items.
    pub documents: usize,
    /// The name typed so far, asked for only when there is something to lose.
    pub typed: &'a str,
    pub handlers: RemoveHandlers,
}

fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

pub fn render_remove(props: RemoveProps<'_>, cx: &App) -> AnyElement {
    let RemoveProps {
        name,
        rooms,
        items,
        documents,
        typed,
        handlers,
    } = props;
    let holds_something = rooms > 0 || items > 0;
    let body: Vec<AnyElement> = if holds_something {
        vec![
            div()
                .text_size(px(13.0))
                .child(crate::msg::desktop_inventory_remove_warning(
                    name,
                    &crate::msg::desktop_inventory_count_rooms(count(rooms)),
                    &crate::msg::desktop_inventory_count_items(count(items)),
                    &crate::msg::desktop_inventory_count_documents(count(documents)),
                ))
                .into_any_element(),
            text_field(
                "remove-property-confirm-input",
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
                dialog::cancel_button("remove-property-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-property-confirm",
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
