//! The **Inventory** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16q, as settled on #491): a
//! kicker with **+ Add property** over one bordered table of Properties in creation order. An
//! expanded Property shows its Rooms in a sub-table that ends in a clickable **+ Add room** row.
//!
//! Every action is a callback into `Shell`, so the keyboard and the mouse reach the same
//! handlers. Selection is one flat list over the visible Property and Room rows; **+ Add room**
//! is never part of it.

use std::{collections::HashSet, rc::Rc};

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_core::{Money, UnitKind};
use lib_locale::format::{Unit, format_money, upper};

use crate::{
    inventory::{Inventory, Property},
    theme::color,
};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnPropertyClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnRowClick = Rc<dyn Fn(InventoryRow, &mut Window, &mut App)>;

/// One selectable row: a Property, or a Room of an expanded Property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRow {
    Property(u32),
    Room(u32),
}

pub struct InventoryPageProps<'a> {
    pub inventory: &'a Inventory,
    /// The Properties shown open.
    pub expanded: &'a HashSet<u32>,
    pub selected: Option<InventoryRow>,
    pub on_add_property_click: OnPlainClick,
    pub on_add_room_click: OnPropertyClick,
    pub on_toggle_click: OnPropertyClick,
    pub on_row_click: OnRowClick,
    pub on_edit_click: OnRowClick,
    pub on_remove_click: OnRowClick,
}

/// The rows `j`/`k` walk, in screen order: each Property, then its Rooms when it is expanded.
pub fn visible_rows(inventory: &Inventory, expanded: &HashSet<u32>) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    for property in &inventory.properties {
        rows.push(InventoryRow::Property(property.id));
        if expanded.contains(&property.id) {
            rows.extend(property.rooms.iter().map(|r| InventoryRow::Room(r.id)));
        }
    }
    rows
}

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The heading's meta as plain text, for the status line: `2 properties · 11 rooms · 232 items`.
pub fn scope_text(inventory: &Inventory) -> String {
    crate::msg::desktop_inventory_scope(
        &crate::msg::desktop_inventory_count_properties(count(inventory.property_count())),
        &crate::msg::desktop_inventory_count_rooms(count(inventory.room_count())),
        &crate::msg::desktop_inventory_count_items(count(inventory.item_count())),
    )
}

/// An amount in the Property's own Unit.
fn money(property: &Property, amount: &Money) -> String {
    let code = property.unit.to_uppercase();
    format_money(amount, &Unit::new(&code, &UnitKind::Fiat, 2))
}

/// The Policy cell: the insurer and, when it has one, the policy number.
fn policy_text(property: &Property) -> Option<String> {
    property.cover.as_ref().map(|cover| match &cover.policy_no {
        Some(number) => format!("{} {number}", cover.insurer),
        None => cover.insurer.clone(),
    })
}

pub fn render(props: &InventoryPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    let table =
        if props.inventory.properties.is_empty() {
            div()
                .mb(px(16.0))
                .px(px(14.0))
                .py(px(14.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(12.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_inventory_empty())
        } else {
            let last = props.inventory.properties.len() - 1;
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .mb(px(16.0))
                .child(table_header(cx))
                .children(props.inventory.properties.iter().enumerate().map(
                    |(position, property)| {
                        property_block(property, position == last, focused, props, cx)
                    },
                ))
        };
    div()
        .flex()
        .flex_col()
        .child(kicker_row(props.on_add_property_click.clone(), cx))
        .child(table)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .max_w(px(640.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_inventory_note_registers())
                .child(crate::msg::desktop_inventory_note_cover())
                .child(crate::msg::desktop_inventory_note_removal()),
        )
        .into_any_element()
}

fn kicker_row(on_add_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_inventory_kicker())),
        )
        .child(
            div()
                .debug_selector(|| "settings-inventory-add-property".to_string())
                .id("settings-inventory-add-property")
                .cursor_pointer()
                .flex_none()
                .py(px(8.0))
                .px(px(14.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add_click(window, cx))
                .child(crate::msg::desktop_inventory_add_property("+")),
        )
}

/// A header cell of a fixed width, right-aligned for the numeric columns.
fn header_cell(label: String, width: f32, right: bool) -> gpui::Div {
    let cell = div().w(px(width)).flex_none().child(upper(&label));
    if right {
        cell.flex().justify_end()
    } else {
        cell
    }
}

fn table_header(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap(px(14.0))
        .px(px(14.0))
        .py(px(6.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        // The disclosure control's width.
        .child(div().w(px(14.0)).flex_none())
        .child(
            div()
                .flex_1()
                .child(upper(&crate::msg::desktop_inventory_column_property())),
        )
        .child(header_cell(
            crate::msg::desktop_inventory_column_policy(),
            170.0,
            false,
        ))
        .child(header_cell(
            crate::msg::desktop_inventory_column_sum_insured(),
            100.0,
            true,
        ))
        .child(header_cell(
            crate::msg::desktop_inventory_column_item_limit(),
            90.0,
            true,
        ))
        .child(header_cell(
            crate::msg::desktop_inventory_column_items(),
            60.0,
            true,
        ))
        .child(header_cell(lib_locale::msg::column_actions(), 150.0, true))
}

/// One Property row and, while it is open, its Room sub-table.
fn property_block(
    property: &Property,
    last: bool,
    focused: bool,
    props: &InventoryPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let open = props.expanded.contains(&property.id);
    div()
        .flex()
        .flex_col()
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(property_row(property, open, focused, props, cx))
        .when(open, |this| {
            this.child(room_table(property, focused, props, cx))
        })
}

fn property_row(
    property: &Property,
    open: bool,
    focused: bool,
    props: &InventoryPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = property.id;
    let row = InventoryRow::Property(id);
    let selected = props.selected == Some(row);
    let on_toggle = props.on_toggle_click.clone();
    // A dash, in the muted colour, where there is no cover or no item limit.
    let placeholder = if selected && focused {
        color::selection_muted(cx)
    } else {
        color::faint_text(cx)
    };
    let optional = |text: Option<String>, width: f32, right: bool| {
        let cell = div().w(px(width)).flex_none().truncate();
        let cell = if right {
            cell.flex().justify_end()
        } else {
            cell
        };
        match text {
            Some(text) => cell.child(text),
            None => cell
                .text_color(placeholder)
                .child(crate::msg::desktop_inventory_none()),
        }
    };
    let cover = property.cover.as_ref();
    let disclosure = if open {
        crate::msg::desktop_inventory_disclosure_open()
    } else {
        crate::msg::desktop_inventory_disclosure_closed()
    };
    selectable_row(
        SharedString::from(format!("settings-inventory-property-{id}")),
        row,
        focused,
        props,
        cx,
    )
    .child(
        div()
            .debug_selector(move || format!("settings-inventory-toggle-{id}"))
            .id(SharedString::from(format!(
                "settings-inventory-toggle-{id}"
            )))
            .cursor_pointer()
            .w(px(14.0))
            .flex_none()
            .on_click(move |_event, window, cx| {
                cx.stop_propagation();
                on_toggle(id, window, cx)
            })
            .child(disclosure),
    )
    .child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .truncate()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(property.name.clone()),
            )
            .child(
                div()
                    .truncate()
                    .text_size(px(11.0))
                    .text_color(placeholder)
                    .child(property.address.clone()),
            ),
    )
    .child(optional(policy_text(property), 170.0, false))
    .child(optional(
        cover.map(|c| money(property, &c.sum_insured)),
        100.0,
        true,
    ))
    .child(optional(
        cover
            .and_then(|c| c.item_limit.as_ref())
            .map(|limit| money(property, limit)),
        90.0,
        true,
    ))
    .child(
        div()
            .w(px(60.0))
            .flex_none()
            .flex()
            .justify_end()
            .child(props.inventory.property_items(id).to_string()),
    )
    .child(actions(row, selected && focused, props, cx))
}

/// The Rooms of an open Property, indented under it, ending in the **+ Add room** row.
fn room_table(
    property: &Property,
    focused: bool,
    props: &InventoryPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = property.id;
    let on_add_room = props.on_add_room_click.clone();
    let hover = color::hover(cx);
    div()
        .flex()
        .flex_col()
        .ml(px(28.0))
        .mb(px(8.0))
        .mr(px(14.0))
        .border_1()
        .border_color(color::border(cx))
        .child(
            div()
                .flex()
                .gap(px(14.0))
                .px(px(14.0))
                .py(px(6.0))
                .bg(color::chrome(cx))
                .border_b(px(1.0))
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::muted(cx))
                .child(
                    div()
                        .flex_1()
                        .child(upper(&crate::msg::desktop_inventory_column_room())),
                )
                .child(header_cell(
                    crate::msg::desktop_inventory_column_items(),
                    60.0,
                    true,
                ))
                .child(header_cell(
                    crate::msg::desktop_inventory_column_value(),
                    100.0,
                    true,
                ))
                .child(header_cell(lib_locale::msg::column_actions(), 118.0, true)),
        )
        .children(property.rooms.iter().map(|room| {
            let row = InventoryRow::Room(room.id);
            let selected = props.selected == Some(row);
            selectable_row(
                SharedString::from(format!("settings-inventory-room-{}", room.id)),
                row,
                focused,
                props,
                cx,
            )
            .border_b(px(1.0))
            .border_color(color::hairline(cx))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(room.name.clone()),
            )
            .child(
                div()
                    .w(px(60.0))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .child(props.inventory.room_items(room.id).to_string()),
            )
            .child(
                div()
                    .w(px(100.0))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .child(money(property, &props.inventory.room_value(room.id))),
            )
            .child(actions(row, selected && focused, props, cx))
        }))
        .child(
            div()
                .debug_selector(move || format!("settings-inventory-add-room-{id}"))
                .id(SharedString::from(format!(
                    "settings-inventory-add-room-{id}"
                )))
                .cursor_pointer()
                .px(px(14.0))
                .py(px(6.0))
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add_room(id, window, cx))
                .child(crate::msg::desktop_inventory_add_room("+")),
        )
}

/// The shared shell of a selectable row. The selected one is inverted while the page has focus
/// (`background:#201e1d; color:#f3f2f2`) and only tinted while the index does.
fn selectable_row(
    id: SharedString,
    row: InventoryRow,
    focused: bool,
    props: &InventoryPageProps<'_>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    let selected = props.selected == Some(row);
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let on_row_click = props.on_row_click.clone();
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(14.0))
        .px(px(14.0))
        .py(px(6.0))
        .text_size(px(12.5))
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_row_click(row, window, cx))
}

/// The buttons of a row: **+ room** (Properties only), edit and remove.
fn actions(
    row: InventoryRow,
    inverted: bool,
    props: &InventoryPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let border: Rgba = if inverted {
        color::selection_muted(cx)
    } else {
        color::border(cx)
    };
    let (kind, id) = match row {
        InventoryRow::Property(id) => ("property", id),
        InventoryRow::Room(id) => ("room", id),
    };
    let on_edit = props.on_edit_click.clone();
    let on_remove = props.on_remove_click.clone();
    // Only a Property row carries **+ room**, and its wider column makes room for it.
    let (width, add_room) = match row {
        InventoryRow::Property(_) => (150.0, Some(props.on_add_room_click.clone())),
        InventoryRow::Room(_) => (118.0, None),
    };
    div()
        .w(px(width))
        .flex_none()
        .flex()
        .justify_end()
        .gap(px(4.0))
        .when_some(add_room, |this, on_add_room| {
            this.child(row_action_button(
                SharedString::from(format!("settings-inventory-property-add-room-{id}")),
                crate::msg::desktop_inventory_row_add_room(),
                border,
                Rc::new(move |window: &mut Window, cx: &mut App| on_add_room(id, window, cx)),
                cx,
            ))
        })
        .child(row_action_button(
            SharedString::from(format!("settings-inventory-edit-{kind}-{id}")),
            crate::msg::desktop_inventory_row_edit(),
            border,
            Rc::new(move |window: &mut Window, cx: &mut App| on_edit(row, window, cx)),
            cx,
        ))
        .child(row_action_button(
            SharedString::from(format!("settings-inventory-remove-{kind}-{id}")),
            crate::msg::desktop_inventory_row_remove(),
            border,
            Rc::new(move |window: &mut Window, cx: &mut App| on_remove(row, window, cx)),
            cx,
        ))
}

/// `padding:4px 8px; font-size:11.5px; border:1px solid`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(8.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.5))
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn seed() -> Inventory {
        crate::inventory::default_inventory(NaiveDate::from_ymd_opt(2026, 10, 2).unwrap())
    }

    #[test]
    fn scope_text_reads_the_live_counts() {
        crate::locale::init_for_tests();
        assert_eq!(
            scope_text(&seed()),
            "2 properties \u{b7} 11 rooms \u{b7} 232 items"
        );
    }

    #[test]
    fn zeros_are_shown_and_one_reads_singular() {
        crate::locale::init_for_tests();
        let mut inventory = seed();
        inventory.properties.truncate(1);
        inventory.items.clear();
        inventory.properties[0].rooms.clear();
        assert_eq!(
            scope_text(&inventory),
            "1 property \u{b7} 0 rooms \u{b7} 0 items"
        );
    }

    #[test]
    fn collapsed_properties_hide_their_rooms() {
        let inventory = seed();
        let rows = visible_rows(&inventory, &HashSet::new());
        assert_eq!(rows.len(), 2);
        let first = &inventory.properties[0];
        let rows = visible_rows(&inventory, &HashSet::from([first.id]));
        assert_eq!(rows.len(), 2 + first.rooms.len());
        assert_eq!(rows[1], InventoryRow::Room(first.rooms[0].id));
    }
}
