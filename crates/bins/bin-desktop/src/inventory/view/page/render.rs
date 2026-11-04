//! The **Inventory** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16q), drawn like
//! Categories from the Presenter's view model: a LOCATIONS · n kicker with a square **+** (Add
//! location) over one bordered NAME table. A Location row has a disclosure, its name in 800 and a
//! muted "n areas · n items" note; an open Location's Areas follow it, indented, name only, in
//! area-tab order.
//!
//! Rows carry no buttons (the Units table principles): a double-click edits, a right-click opens
//! the row's context menu at the pointer. Every action is an event to the page's Presenter, so the
//! keyboard and the mouse reach the same handlers.

use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, Context, MouseButton, MouseDownEvent, Pixels, Point,
    Render, SharedString, Window, div, point, prelude::*, px,
};
use lib_core::LocationId;
use lib_inventory::InventoryRow;
use lib_inventory::presenter::{
    InventoryLine, InventoryPageEvent, InventoryPageViewModel, MenuView,
};

use super::{InventoryPage, slug};
use crate::{shell::view::page_clicks::PageClicks, theme::color};

/// The context menu's width (`width:190px`).
const MENU_WIDTH: f32 = 190.0;

impl Render for InventoryPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let view = self.view_model();
        let clicks = PageClicks::new(cx);
        render_page(&view, self.menu_at, &clicks, self.page_focused, cx)
    }
}

fn render_page(
    view: &InventoryPageViewModel,
    menu_at: Point<Pixels>,
    clicks: &PageClicks<InventoryPage>,
    focused: bool,
    cx: &App,
) -> AnyElement {
    let table = if view.rows.is_empty() {
        div()
            .mb(px(12.0))
            .px(px(14.0))
            .py(px(14.0))
            .border_1()
            .border_color(color::border(cx))
            .text_size(px(12.5))
            .text_color(color::muted(cx))
            .child(view.empty.clone())
    } else {
        let last = view.rows.len().saturating_sub(1);
        div()
            .flex()
            .flex_col()
            .border_1()
            .border_color(color::border(cx))
            .mb(px(12.0))
            .child(table_header(&view.column_name, cx))
            .children(view.rows.iter().enumerate().map(|(position, line)| {
                tree_row(line, position == last, focused, view, clicks, cx)
            }))
    };
    div()
        .flex()
        .flex_col()
        .child(kicker_row(view, clicks, cx))
        .child(table)
        .child(
            div()
                .max_w(px(620.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(view.note.clone()),
        )
        .children(
            view.menu
                .as_ref()
                .map(|menu| row_menu(menu, menu_at, clicks, cx)),
        )
        .into_any_element()
}

/// LOCATIONS · n, with the square **+** (`26px`, `background:#eae9e9`) that adds a Location.
fn kicker_row(
    view: &InventoryPageViewModel,
    clicks: &PageClicks<InventoryPage>,
    cx: &App,
) -> impl IntoElement {
    let on_add_click = clicks.row(InventoryPageEvent::New);
    let hover = color::hover(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .mb(px(8.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(view.kicker.clone()),
        )
        .child(
            div()
                .debug_selector(|| "settings-inventory-add-location".to_string())
                .id("settings-inventory-add-location")
                .cursor_pointer()
                .flex_none()
                .size(px(26.0))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .text_color(color::foreground(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .hover(move |style| style.bg(hover))
                .on_click(on_add_click)
                .child(view.add_button.clone()),
        )
}

/// The NAME header (`padding:6px 14px; background:#eae9e9`).
fn table_header(name: &str, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px(px(14.0))
        .py(px(6.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(div().flex_1().child(name.to_string()))
}

/// One visible row: a Location (disclosure, name in 800, note) or one of its Areas (indented 20px,
/// name only). `padding:6px 14px; font-size:12.5px`; the selected row is inverted while the page
/// has focus (`background:#201e1d; color:#f3f2f2`) and only tinted while the index does.
fn tree_row(
    line: &InventoryLine,
    last: bool,
    focused: bool,
    view: &InventoryPageViewModel,
    clicks: &PageClicks<InventoryPage>,
    cx: &App,
) -> AnyElement {
    let row = line.row;
    let selected = view.selected == Some(row);
    let inverted = selected && focused;
    let note_colour = if inverted {
        color::selection_muted(cx)
    } else {
        color::faint_text(cx)
    };
    let (id, depth, name) = match row {
        InventoryRow::Location(id) => {
            let name = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .min_w(px(0.0))
                .child(disclosure(
                    id,
                    line.disclosure.clone().unwrap_or_default(),
                    inverted,
                    clicks,
                    cx,
                ))
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(line.name.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.5))
                        .text_color(note_colour)
                        .child(line.note.clone().unwrap_or_default()),
                );
            (format!("settings-inventory-location-{id}"), 0.0, name)
        }
        InventoryRow::Area(id) => {
            let name = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .min_w(px(0.0))
                // The disclosure's width, empty, so Area names line up under their Location's.
                .child(div().w(px(10.0)).flex_none())
                .child(div().truncate().child(line.name.clone()));
            (format!("settings-inventory-area-{id}"), 20.0, name)
        }
    };
    let id = SharedString::from(id);
    let hover = color::hover(cx);
    let on_row_click = clicks.row(InventoryPageEvent::Select(row));
    let on_row_double_click = clicks.row(InventoryPageEvent::EditRow(row));
    let on_row_menu = clicks.menu(InventoryPageEvent::OpenMenu(row));
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(6.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |event: &ClickEvent, window, cx| {
            if event.click_count() >= 2 {
                on_row_double_click(event, window, cx);
            } else {
                on_row_click(event, window, cx);
            }
        })
        .on_mouse_down(
            MouseButton::Right,
            move |event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                on_row_menu(event, window, cx);
            },
        )
        .child(div().flex_1().min_w(px(0.0)).pl(px(depth)).child(name))
        .into_any_element()
}

/// The 10px ▾/▸ cell of a Location row. Only it expands and collapses; a click on it never
/// reaches the row.
fn disclosure(
    id: LocationId,
    glyph: String,
    inverted: bool,
    clicks: &PageClicks<InventoryPage>,
    cx: &App,
) -> impl IntoElement {
    let on_click = clicks.row(InventoryPageEvent::Toggle(id));
    div()
        .debug_selector(move || format!("settings-inventory-toggle-{id}"))
        .id(SharedString::from(format!(
            "settings-inventory-toggle-{id}"
        )))
        .cursor_pointer()
        .w(px(10.0))
        .flex_none()
        .text_color(if inverted {
            color::selection_muted(cx)
        } else {
            color::muted(cx)
        })
        .on_click(move |event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            on_click(event, window, cx);
        })
        .child(glyph)
}

/// The row's context menu at the pointer: `190px`, `border:2px solid #201e1d`, the dialog shadow,
/// items `padding:7px 12px; 12.5px`, the hovered one tinted and in 800, the destructive one last
/// in red below a 1px rule. A press anywhere outside it closes it.
fn row_menu(
    menu: &MenuView,
    at: Point<Pixels>,
    clicks: &PageClicks<InventoryPage>,
    cx: &App,
) -> AnyElement {
    let on_dismiss = clicks.send(InventoryPageEvent::CloseMenu);
    let hover = color::hover(cx);
    let items = menu.entries.iter().map(|entry| {
        let action = entry.action;
        let on_action = clicks.send(InventoryPageEvent::Menu(action));
        let id = SharedString::from(format!("settings-inventory-menu-{}", slug(action)));
        let item = div()
            .debug_selector({
                let id = id.clone();
                move || id.to_string()
            })
            .id(id)
            .cursor_pointer()
            .py(px(7.0))
            .px(px(12.0))
            .text_size(px(12.5))
            .text_color(if entry.destructive {
                color::accent_text(cx)
            } else {
                color::foreground(cx)
            })
            .hover(move |style| style.bg(hover).font_weight(gpui::FontWeight::EXTRA_BOLD))
            .on_click(on_action)
            .child(entry.label.clone());
        if entry.destructive {
            div()
                .flex()
                .flex_col()
                .child(div().h(px(1.0)).my(px(4.0)).bg(color::hairline(cx)))
                .child(item)
                .into_any_element()
        } else {
            item.into_any_element()
        }
    });
    gpui::deferred(
        gpui::anchored().position(at).snap_to_window().child(
            div()
                .debug_selector(|| "settings-inventory-menu".to_string())
                .id("settings-inventory-menu")
                .occlude()
                .w(px(MENU_WIDTH))
                .flex()
                .flex_col()
                .py(px(4.0))
                .bg(color::background(cx))
                .border_2()
                .border_color(color::foreground(cx))
                .shadow(vec![BoxShadow {
                    color: color::dialog_shadow(cx).into(),
                    offset: point(px(0.0), px(16.0)),
                    blur_radius: px(48.0),
                    spread_radius: px(0.0),
                }])
                .font_weight(gpui::FontWeight::NORMAL)
                .on_mouse_down_out(on_dismiss)
                .children(items),
        ),
    )
    .with_priority(1)
    .into_any_element()
}
