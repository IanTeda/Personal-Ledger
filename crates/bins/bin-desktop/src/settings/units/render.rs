//! Draws the **Units** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16g): a UNITS title
//! with a square **+** (Add unit) over a table of ◆ (the base unit) · CODE · NAME · TYPE · PRICE
//! SOURCE, the `◆ base unit` legend, then PRICE SOURCES with its own **+** over NAME · SOURCE ·
//! LAST UPDATED.
//!
//! Rows carry no buttons (the Units table principles): a double-click edits a unit, a right-click
//! opens the row's context menu at the pointer, and the row takes the selection treatment while
//! its menu is open. Every action is a callback into [`UnitsPage`], so the keyboard and the mouse
//! reach the same handlers. Price Sources' test, edit, delete and add have **no dialog ticket** on
//! this map, so they flash a plain "not yet built" status message.

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, Context, Entity, MouseButton, Pixels, Point, SharedString, Window,
    div, point, prelude::*, px,
};
use lib_locale::Label;
use lib_units::{PriceSourceRow, UnitId, UnitRow};

use super::{MenuAction, MenuTarget, RowMenu, UnitsPage};
use crate::{settings::SettingsPage, theme::color};

type OnRowIndexClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRowMenu = Rc<dyn Fn(MenuTarget, Point<Pixels>, &mut Window, &mut App)>;
type OnMenuAction = Rc<dyn Fn(MenuAction, &mut Window, &mut App)>;

/// The context menu's width (`width:190px`).
const MENU_WIDTH: f32 = 190.0;
/// The square **+** beside each table title (`26px`).
const ADD_BUTTON_SIZE: Pixels = px(26.0);

/// What a table's rows do on a click, a double-click and a right-click.
#[derive(Clone)]
struct RowHandlers {
    select: OnRowIndexClick,
    edit: OnRowIndexClick,
    menu: OnRowMenu,
}

impl Render for UnitsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let docked = self.docked;
        let selected = if docked { self.selected_id(cx) } else { None };
        let menu = self.menu;
        let on_menu: OnRowMenu = {
            let entity = entity.clone();
            Rc::new(move |target, position, _window, cx| {
                entity.update(cx, |page, cx| page.open_menu(target, position, cx));
            })
        };
        let unit_handlers = RowHandlers {
            select: on_row(&entity, UnitsPage::click_select),
            edit: on_row(&entity, UnitsPage::double_click_unit),
            menu: on_menu.clone(),
        };
        let source_handlers = RowHandlers {
            select: row_stub(&entity),
            edit: row_stub_message(
                &entity,
                crate::msg::desktop_status_edit_price_source_not_yet_built,
            ),
            menu: on_menu,
        };
        let on_action: OnMenuAction = {
            let entity = entity.clone();
            Rc::new(move |action, _window, cx| {
                entity.update(cx, |page, cx| page.click_menu_action(action, cx));
            })
        };
        let on_dismiss: OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, |page, cx| {
                    page.close_menu(cx);
                });
            })
        };
        let cx: &App = cx;
        div()
            .flex()
            .flex_col()
            .child(table_title(
                lib_locale::format::upper(&SettingsPage::Units.label()),
                "settings-add-unit",
                on_plain(&entity, UnitsPage::click_add),
                cx,
            ))
            .child(table(self.units(cx), unit_handlers, selected, menu, cx))
            .child(legend(cx))
            .child(
                div()
                    .mt(px(32.0))
                    .flex()
                    .flex_col()
                    .child(table_title(
                        lib_locale::format::upper(
                            &crate::msg::desktop_settings_price_sources_heading(),
                        ),
                        "settings-add-price-source",
                        stub(
                            &entity,
                            crate::msg::desktop_status_add_price_source_not_yet_built,
                        ),
                        cx,
                    ))
                    .child(price_source_table(
                        self.price_sources(cx),
                        source_handlers,
                        menu,
                        cx,
                    )),
            )
            .children(menu.map(|menu| row_menu(menu, on_action, on_dismiss, cx)))
    }
}

/// A click handler that runs `act` on the page with the clicked row's index.
fn on_row(
    entity: &Entity<UnitsPage>,
    act: fn(&mut UnitsPage, usize, &mut Context<'_, UnitsPage>),
) -> OnRowIndexClick {
    let entity = entity.clone();
    Rc::new(move |index, _window, cx| entity.update(cx, |page, cx| act(page, index, cx)))
}

/// As [`on_row`], for a click that carries no row.
fn on_plain(
    entity: &Entity<UnitsPage>,
    act: fn(&mut UnitsPage, &mut Context<'_, UnitsPage>),
) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, act))
}

/// As [`on_plain`], for a button with nothing behind it yet: it says so on the status line.
fn stub(entity: &Entity<UnitsPage>, message: fn() -> String) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, |page, cx| page.not_built(message(), cx)))
}

/// As [`stub`], for a row's double-click, whose row it ignores.
fn row_stub_message(entity: &Entity<UnitsPage>, message: fn() -> String) -> OnRowIndexClick {
    let entity = entity.clone();
    Rc::new(move |_index, _window, cx| entity.update(cx, |page, cx| page.not_built(message(), cx)))
}

/// A Price Sources row's plain click: it selects nothing and only closes an open menu.
fn row_stub(entity: &Entity<UnitsPage>) -> OnRowIndexClick {
    let entity = entity.clone();
    Rc::new(move |_index, _window, cx| {
        entity.update(cx, |page, cx| {
            page.close_menu(cx);
        });
    })
}

/// A table's caps title (`font:800 10px/1; letter-spacing:.11em; color:#9b9797`) sharing its row
/// with a square **+** at the table's right edge.
fn table_title(
    text: String,
    id: &'static str,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
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
                .child(text),
        )
        .child(
            div()
                .debug_selector(move || id.to_string())
                .id(id)
                .cursor_pointer()
                .size(ADD_BUTTON_SIZE)
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child("+"),
        )
}

/// `◆ base unit`, aligned right under the Units table.
fn legend(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_end()
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(div().text_color(color::accent(cx)).child(BASE_GLYPH))
                .child(crate::msg::desktop_settings_units_legend_base()),
        )
}

/// The base unit's marker, in the table's unheaded glyph column.
const BASE_GLYPH: &str = "\u{25c6}";

const GLYPH_WIDTH: Pixels = px(24.0);
const CODE_WIDTH: Pixels = px(100.0);
const TYPE_WIDTH: Pixels = px(90.0);
const SOURCE_WIDTH: Pixels = px(130.0);

/// Where a Units row sits and how it is drawn.
#[derive(Clone, Copy)]
struct RowState {
    index: usize,
    last: bool,
    /// Whether the row takes the selection treatment: the selected row at X-Large, or the row a
    /// menu is open on.
    inverted: bool,
}

fn table(
    units: &[UnitRow],
    handlers: RowHandlers,
    selected: Option<UnitId>,
    menu: Option<RowMenu>,
    cx: &App,
) -> impl IntoElement {
    let last_index = units.len().saturating_sub(1);
    let menu_unit = menu.and_then(|menu| match menu.target {
        MenuTarget::Unit(id) => Some(id),
        MenuTarget::Source(_) => None,
    });
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(cx))
        .children(units.iter().enumerate().map(|(index, unit)| {
            let state = RowState {
                index,
                last: index == last_index,
                inverted: selected == Some(unit.id) || menu_unit == Some(unit.id),
            };
            row(unit, state, handlers.clone(), cx)
        }))
}

fn header_cell(width: Option<Pixels>, text: Option<String>) -> impl IntoElement {
    let cell = div();
    let cell = match width {
        Some(width) => cell.w(width).flex_none(),
        None => cell.flex_1(),
    };
    cell.children(text.map(|text| lib_locale::format::upper(&text)))
}

fn header_row(cx: &App) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
}

fn table_header(cx: &App) -> impl IntoElement {
    header_row(cx)
        .child(header_cell(Some(GLYPH_WIDTH), None))
        .child(header_cell(
            Some(CODE_WIDTH),
            Some(lib_locale::msg::column_code()),
        ))
        .child(header_cell(None, Some(lib_locale::msg::column_name())))
        .child(header_cell(
            Some(TYPE_WIDTH),
            Some(lib_locale::msg::column_type()),
        ))
        .child(header_cell(
            Some(SOURCE_WIDTH),
            Some(crate::msg::desktop_settings_units_column_price_source()),
        ))
}

/// A row's frame: `padding:12px 16px`, a hairline under every row but the last, the selection
/// treatment when `inverted`, and a double-click and right-click bound to it.
fn row_frame(
    id: SharedString,
    last: bool,
    inverted: bool,
    on_click: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_menu: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(move |event, window, cx| on_click(event.click_count(), window, cx))
        .on_mouse_down(MouseButton::Right, move |event, window, cx| {
            cx.stop_propagation();
            on_menu(event.position, window, cx);
        })
}

fn row(unit: &UnitRow, state: RowState, handlers: RowHandlers, cx: &App) -> impl IntoElement {
    let RowState {
        index,
        last,
        inverted,
    } = state;
    let muted = if inverted {
        color::selection_muted(cx)
    } else {
        color::faint_text(cx)
    };
    let id = unit.id;
    let RowHandlers { select, edit, menu } = handlers;
    row_frame(
        SharedString::from(format!("unit-row-{index}")),
        last,
        inverted,
        move |clicks, window, cx| {
            if clicks >= 2 {
                edit(index, window, cx);
            } else {
                select(index, window, cx);
            }
        },
        move |position, window, cx| menu(MenuTarget::Unit(id), position, window, cx),
        cx,
    )
    .child(
        div()
            .w(GLYPH_WIDTH)
            .flex_none()
            .text_size(px(12.0))
            .when(unit.is_base, |this| {
                this.text_color(if inverted {
                    color::selection_text(cx)
                } else {
                    color::accent(cx)
                })
                .child(BASE_GLYPH)
            }),
    )
    .child(
        div()
            .w(CODE_WIDTH)
            .flex_none()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(unit.code.clone()),
    )
    .child(div().flex_1().child(unit.name.clone()))
    .child(
        div()
            .w(TYPE_WIDTH)
            .flex_none()
            .text_color(muted)
            .child(unit.kind.label()),
    )
    .child(
        div()
            .w(SOURCE_WIDTH)
            .flex_none()
            .text_color(muted)
            .child(unit.source.clone()),
    )
}

const PRICE_SOURCE_NAME_WIDTH: Pixels = px(130.0);
const PRICE_SOURCE_LAST_UPDATED_WIDTH: Pixels = px(130.0);

fn price_source_table(
    rows: &[PriceSourceRow],
    handlers: RowHandlers,
    menu: Option<RowMenu>,
    cx: &App,
) -> impl IntoElement {
    let last_index = rows.len().saturating_sub(1);
    let menu_index = menu.and_then(|menu| match menu.target {
        MenuTarget::Source(index) => Some(index),
        MenuTarget::Unit(_) => None,
    });
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .child(
            header_row(cx)
                .child(header_cell(
                    Some(PRICE_SOURCE_NAME_WIDTH),
                    Some(lib_locale::msg::column_name()),
                ))
                .child(header_cell(
                    None,
                    Some(crate::msg::desktop_settings_units_column_source()),
                ))
                .child(header_cell(
                    Some(PRICE_SOURCE_LAST_UPDATED_WIDTH),
                    Some(crate::msg::desktop_settings_units_column_last_updated()),
                )),
        )
        .children(rows.iter().enumerate().map(|(index, source)| {
            price_source_row(
                source,
                index,
                index == last_index,
                menu_index == Some(index),
                handlers.clone(),
                cx,
            )
        }))
}

/// One Price Sources row -- keyed by the unit's own `name`, not `code` (the README's own "the
/// one place the unit is referenced by name rather than code" callout, since this table is
/// about the relationship to an external source, not the unit record itself).
fn price_source_row(
    source: &PriceSourceRow,
    index: usize,
    last: bool,
    inverted: bool,
    handlers: RowHandlers,
    cx: &App,
) -> impl IntoElement {
    let RowHandlers { select, edit, menu } = handlers;
    row_frame(
        SharedString::from(format!("price-source-row-{index}")),
        last,
        inverted,
        move |clicks, window, cx| {
            if clicks >= 2 {
                edit(index, window, cx);
            } else {
                select(index, window, cx);
            }
        },
        move |position, window, cx| menu(MenuTarget::Source(index), position, window, cx),
        cx,
    )
    .child(
        div()
            .w(PRICE_SOURCE_NAME_WIDTH)
            .flex_none()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(source.unit_name.clone()),
    )
    .child(div().flex_1().child(source.source.clone()))
    .child(
        div()
            .w(PRICE_SOURCE_LAST_UPDATED_WIDTH)
            .flex_none()
            .text_color(if inverted {
                color::selection_muted(cx)
            } else {
                color::faint_text(cx)
            })
            .child(source.last_updated.clone()),
    )
}

/// The row's context menu at the pointer: `190px`, `border:2px solid #201e1d`, the dialog shadow,
/// items `padding:7px 12px; 12.5px`, the hovered one tinted and in 800, the destructive one last
/// in red below a 1px rule. A press anywhere outside it closes it.
fn row_menu(
    menu: RowMenu,
    on_action: OnMenuAction,
    on_dismiss: OnPlainClick,
    cx: &App,
) -> AnyElement {
    let hover = color::hover(cx);
    let items = MenuAction::for_target(menu.target).iter().map(|&action| {
        let on_action = on_action.clone();
        let id = SharedString::from(format!("settings-units-menu-{}", action.slug()));
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
            .text_color(if action.is_destructive() {
                color::accent_text(cx)
            } else {
                color::foreground(cx)
            })
            .hover(move |style| style.bg(hover).font_weight(gpui::FontWeight::EXTRA_BOLD))
            .on_click(move |_event, window, cx| on_action(action, window, cx))
            .child(action.label());
        if action.is_destructive() {
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
        gpui::anchored()
            .position(menu.position)
            .snap_to_window()
            .child(
                div()
                    .debug_selector(|| "settings-units-menu".to_string())
                    .id("settings-units-menu")
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
                    .on_mouse_down_out(move |_event, window, cx| on_dismiss(window, cx))
                    .children(items),
            ),
    )
    .with_priority(1)
    .into_any_element()
}

// ---------------------------------------------------------------------------------------------
// The X-Large detail pane (DESKTOP-125)
// ---------------------------------------------------------------------------------------------

/// `width:400px` at the View's right edge.
const PANE_WIDTH: Pixels = px(400.0);
/// Every pane button is 112 x 34px, so three fill the 368px row with 8px gaps.
const PANE_BUTTON_WIDTH: Pixels = px(112.0);
const PANE_BUTTON_HEIGHT: Pixels = px(34.0);

/// The pane docked beside the page at X-Large: the selected unit, its price source and this
/// session's outcomes. It shows and offers the actions; the Dialogs edit.
pub(crate) fn detail_pane(page: &UnitsPage, entity: &Entity<UnitsPage>, cx: &App) -> AnyElement {
    let pane = div()
        .debug_selector(|| "units-detail-pane".to_string())
        .id("units-detail-pane")
        .w(PANE_WIDTH)
        .flex_none()
        .h_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx));
    let unit = page.selected_unit(cx);
    pane.when_some(unit, |this, unit| {
        let source = page.price_source_of(unit, cx);
        this.child(unit_group(page, unit, entity, cx))
            .child(price_source_group(source, entity, cx))
            .child(accounts_group(page, unit, cx))
    })
    .child(activity_group(page, cx))
    .into_any_element()
}

/// A group: a caps label over a 2px rule, then the body.
fn pane_group(
    label: String,
    body: impl IntoElement,
    id: &'static str,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(move || id.to_string())
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(16.0))
                .pt(px(14.0))
                .pb(px(8.0))
                .border_b(px(2.0))
                .border_color(color::structural_rule(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .child(lib_locale::format::upper(&label)),
        )
        .child(
            div()
                .px(px(16.0))
                .pt(px(12.0))
                .pb(px(16.0))
                .flex()
                .flex_col()
                .child(body),
        )
}

/// `padding:5px 0; border-bottom:1px solid` hairline; the label muted, the value right-aligned.
fn fact_row(label: String, value: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .py(px(5.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.0))
        .child(div().text_color(color::muted(cx)).child(label))
        .child(div().text_align(gpui::TextAlign::Right).child(value))
}

fn pane_button(
    id: &'static str,
    label: String,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(move || id.to_string())
        .id(id)
        .cursor_pointer()
        .w(PANE_BUTTON_WIDTH)
        .h(PANE_BUTTON_HEIGHT)
        .flex()
        .items_center()
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .bg(color::background(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

fn button_row(buttons: impl IntoIterator<Item = AnyElement>) -> impl IntoElement {
    div().flex().gap(px(8.0)).mt(px(12.0)).children(buttons)
}

fn unit_group(
    page: &UnitsPage,
    unit: &UnitRow,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let base = page
        .units(cx)
        .iter()
        .find(|row| row.is_base)
        .map_or_else(|| unit.code.clone(), |row| row.code.clone());
    let flags = [
        unit.is_base
            .then(crate::msg::desktop_settings_units_flag_base),
        unit.is_default
            .then(crate::msg::desktop_settings_units_flag_default),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let flags = if flags.is_empty() {
        crate::msg::desktop_settings_units_pane_none()
    } else {
        flags.join(" \u{b7} ")
    };
    let index = page.selected_index(cx).unwrap_or_default();
    let accounts = page.accounts_in(unit, cx);
    let transactions: u32 = accounts
        .iter()
        .map(|account| account.transaction_count)
        .sum();
    let count = |len: usize| i64::try_from(len).unwrap_or(i64::MAX);
    let body = div()
        .flex()
        .flex_col()
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .line_height(gpui::relative(1.25))
                .child(crate::msg::desktop_settings_units_pane_title(
                    &unit.code, &unit.name,
                )),
        )
        .child(
            div()
                .mb(px(8.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_units_pane_meta(
                    &unit.kind.label(),
                    &base,
                )),
        )
        .child(fact_row(
            crate::msg::desktop_settings_units_column_flags(),
            flags,
            cx,
        ))
        .child(fact_row(
            crate::msg::desktop_settings_units_pane_used_by(),
            crate::msg::desktop_settings_units_pane_used_by_value(
                count(accounts.len()),
                i64::from(transactions),
            ),
            cx,
        ))
        .child(button_row([
            pane_button(
                "units-pane-edit-unit",
                crate::msg::desktop_settings_units_pane_edit_unit(),
                row_click(entity, index, UnitsPage::click_edit),
                cx,
            )
            .into_any_element(),
            pane_button(
                "units-pane-delete-unit",
                crate::msg::desktop_settings_units_pane_delete_unit(),
                row_click(entity, index, UnitsPage::click_delete),
                cx,
            )
            .into_any_element(),
        ]));
    pane_group(
        crate::msg::desktop_settings_units_pane_unit(),
        body,
        "units-pane-unit",
        cx,
    )
}

fn price_source_group(
    source: Option<&PriceSourceRow>,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let body = match source {
        Some(source) => div()
            .flex()
            .flex_col()
            .child(fact_row(
                crate::msg::desktop_settings_units_column_source(),
                source.source.clone(),
                cx,
            ))
            .child(fact_row(
                crate::msg::desktop_settings_units_column_last_updated(),
                source.last_updated.clone(),
                cx,
            ))
            .child(button_row([
                pane_button(
                    "units-pane-test-source",
                    crate::msg::desktop_settings_units_pane_test(),
                    stub(
                        entity,
                        crate::msg::desktop_status_test_price_source_not_yet_built,
                    ),
                    cx,
                )
                .into_any_element(),
                pane_button(
                    "units-pane-edit-source",
                    crate::msg::desktop_settings_units_pane_edit_source(),
                    stub(
                        entity,
                        crate::msg::desktop_status_edit_price_source_not_yet_built,
                    ),
                    cx,
                )
                .into_any_element(),
                pane_button(
                    "units-pane-delete-source",
                    crate::msg::desktop_settings_units_pane_delete_source(),
                    stub(
                        entity,
                        crate::msg::desktop_status_delete_price_source_not_yet_built,
                    ),
                    cx,
                )
                .into_any_element(),
            ])),
        // A unit without a source offers to add one.
        None => div().flex().flex_col().child(button_row([pane_button(
            "units-pane-add-source",
            crate::msg::desktop_settings_units_pane_add_source(),
            stub(
                entity,
                crate::msg::desktop_status_add_price_source_not_yet_built,
            ),
            cx,
        )
        .into_any_element()])),
    };
    pane_group(
        crate::msg::desktop_settings_price_sources_heading(),
        body,
        "units-pane-source",
        cx,
    )
}

/// The accounts held in the selected unit, one 12px row each: name over type on the left, the
/// balance in the unit's own precision on the right, and a bold **Total** closing the list. The
/// base-unit value is not drawn: the ledger holds no price to convert by yet.
fn accounts_group(page: &UnitsPage, unit: &UnitRow, cx: &App) -> impl IntoElement {
    let accounts = page.accounts_in(unit, cx);
    let body = if accounts.is_empty() {
        div()
            .text_size(px(11.0))
            .text_color(color::faint_text(cx))
            .child(crate::msg::desktop_settings_units_pane_accounts_none())
            .into_any_element()
    } else {
        let balance = |money: &lib_core::Money| {
            format!("{} {}", crate::view::format::amount(money).1, unit.code)
        };
        let total = lib_core::Money(
            accounts
                .iter()
                .map(|account| account.balance.0.clone())
                .sum(),
        );
        div()
            .flex()
            .flex_col()
            .children(accounts.iter().map(|account| {
                div()
                    .flex()
                    .justify_between()
                    .gap(px(12.0))
                    .py(px(5.0))
                    .border_b(px(1.0))
                    .border_color(color::hairline(cx))
                    .text_size(px(12.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                    .child(account.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(color::faint_text(cx))
                                    .child(account.account_type.label()),
                            ),
                    )
                    .child(balance(&account.balance))
            }))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap(px(12.0))
                    .pt(px(8.0))
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(crate::msg::desktop_settings_units_pane_accounts_total())
                    .child(balance(&total)),
            )
            .into_any_element()
    };
    pane_group(
        crate::msg::desktop_settings_units_pane_accounts(),
        body,
        "units-pane-accounts",
        cx,
    )
}

/// This session's outcomes, newest first, each a 12px row with a 38px tabular time.
fn activity_group(page: &UnitsPage, cx: &App) -> impl IntoElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .children(page.activity.iter().map(|entry| {
            div()
                .flex()
                .gap(px(8.0))
                .text_size(px(12.0))
                .child(
                    div()
                        .w(px(38.0))
                        .flex_none()
                        .text_color(color::muted(cx))
                        .child(entry.time.clone()),
                )
                .child(div().child(entry.text.clone()))
        }))
        .child(
            div()
                .mt(px(6.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_settings_units_pane_activity_note()),
        );
    pane_group(
        crate::msg::desktop_settings_units_pane_activity(),
        body,
        "units-pane-activity",
        cx,
    )
}

/// A plain click that runs `act` for the row at `index`.
fn row_click(
    entity: &Entity<UnitsPage>,
    index: usize,
    act: fn(&mut UnitsPage, usize, &mut Context<'_, UnitsPage>),
) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, |page, cx| act(page, index, cx)))
}
