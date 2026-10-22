//! Draws the Units page (`docs/ux/desktop-mockups/16-settings/README.md`'s 16g) from its view
//! model: a UNITS title with a square **+** over ◆ (the base unit) · CODE · NAME · TYPE · PRICE
//! SOURCE, the `◆ base unit` legend, then PRICE SOURCES with its own **+** over NAME · SOURCE ·
//! LAST UPDATED.
//!
//! Rows carry no buttons: a click selects, a double-click edits and a right-click opens the row's
//! menu at the pointer. Every click is an event to the Presenter.

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, Context, Entity, MouseButton, Pixels, Point, SharedString, Window,
    div, point, prelude::*, px,
};
use lib_units::presenter::{
    MenuEntry, NOT_FETCHED_GLYPH, RowMenu, SourceColumns, SourceKind, SourceLine, UnitColumns,
    UnitLine, UnitsPageEvent, UnitsPageViewModel, UnitsRow,
};

use super::UnitsPage;
use crate::{settings::SettingsPage, theme::color};

/// The context menu's width (`width:190px`).
const MENU_WIDTH: f32 = 190.0;
/// The narrowest both tables draw: the width the page gets in a window at Medium's 720px floor
/// (ADR-0042). Any wider window, up to X-Large beside the docked pane, they fill.
const TABLE_MIN_WIDTH: Pixels = px(610.0);
/// The square **+** beside each table title (`26px`).
const ADD_BUTTON_SIZE: Pixels = px(26.0);
/// The base unit's marker, in the table's unheaded glyph column. A glyph, not a Message.
const BASE_GLYPH: &str = "\u{25c6}";

const GLYPH_WIDTH: Pixels = px(24.0);
const CODE_WIDTH: Pixels = px(100.0);
const TYPE_WIDTH: Pixels = px(90.0);
const SOURCE_WIDTH: Pixels = px(130.0);
const PRICE_SOURCE_NAME_WIDTH: Pixels = px(130.0);
const PRICE_SOURCE_LAST_UPDATED_WIDTH: Pixels = px(130.0);

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnRowMenu = Rc<dyn Fn(Point<Pixels>, &mut Window, &mut App)>;

/// A click that sends `event` to the page.
pub(super) fn send(entity: &Entity<UnitsPage>, event: UnitsPageEvent) -> OnClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| {
        let event = event.clone();
        entity.update(cx, |page, cx| page.send(event, cx));
    })
}

/// A row's click: a double-click activates it, a single click selects it.
fn row_click(entity: &Entity<UnitsPage>, row: UnitsRow) -> OnRowClick {
    let entity = entity.clone();
    Rc::new(move |clicks, _window, cx| {
        let event = if clicks >= 2 {
            UnitsPageEvent::Activate(row)
        } else {
            UnitsPageEvent::Select(row)
        };
        entity.update(cx, |page, cx| page.click(event, cx));
    })
}

fn row_menu_click(entity: &Entity<UnitsPage>, row: UnitsRow) -> OnRowMenu {
    let entity = entity.clone();
    Rc::new(move |position, _window, cx| {
        entity.update(cx, |page, cx| page.open_menu(row, position, cx));
    })
}

impl Render for UnitsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model(cx);
        let menu_at = self.menu_at;
        let cx: &App = cx;
        let UnitsPageViewModel {
            unit_columns,
            units,
            price_sources_heading,
            source_columns,
            price_sources,
            base_legend,
            menu,
            ..
        } = view;
        div()
            .flex()
            .flex_col()
            .child(table_title(
                lib_locale::format::upper(&SettingsPage::Units.label()),
                "settings-add-unit",
                send(&entity, UnitsPageEvent::NewUnit),
                cx,
            ))
            .child(units_table(&unit_columns, &units, &entity, cx))
            .child(legend(base_legend, cx))
            .child(
                div()
                    .mt(px(32.0))
                    .flex()
                    .flex_col()
                    .child(table_title(
                        lib_locale::format::upper(&price_sources_heading),
                        "settings-add-price-source",
                        send(&entity, UnitsPageEvent::NewPriceSource),
                        cx,
                    ))
                    .child(sources_table(&source_columns, &price_sources, &entity, cx)),
            )
            .children(menu.map(|menu| row_menu(menu, menu_at, &entity, cx)))
    }
}

/// A table's caps title sharing its row with a square **+** at the table's right edge.
fn table_title(text: String, id: &'static str, on_click: OnClick, cx: &App) -> impl IntoElement {
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
fn legend(text: String, cx: &App) -> impl IntoElement {
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
                .child(text),
        )
}

fn header_cell(width: Option<Pixels>, text: Option<&str>) -> impl IntoElement {
    let cell = div();
    let cell = match width {
        Some(width) => cell.w(width).flex_none(),
        None => cell.flex_1(),
    };
    cell.children(text.map(lib_locale::format::upper))
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

fn table_frame(selector: &'static str, cx: &App) -> gpui::Div {
    div()
        .debug_selector(move || selector.to_string())
        .min_w(TABLE_MIN_WIDTH)
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::border(cx))
}

/// A row's frame: `padding:12px 16px`, a hairline under every row but the last, the selection
/// treatment when `inverted`, and its click and right-click.
fn row_frame(
    id: SharedString,
    last: bool,
    inverted: bool,
    on_click: OnRowClick,
    on_menu: OnRowMenu,
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

fn units_table(
    columns: &UnitColumns,
    units: &[UnitLine],
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let last = units.len().saturating_sub(1);
    table_frame("settings-units-table", cx)
        .child(
            header_row(cx)
                .child(header_cell(Some(GLYPH_WIDTH), None))
                .child(header_cell(Some(CODE_WIDTH), Some(&columns.code)))
                .child(header_cell(None, Some(&columns.name)))
                .child(header_cell(Some(TYPE_WIDTH), Some(&columns.kind)))
                .child(header_cell(Some(SOURCE_WIDTH), Some(&columns.price_source))),
        )
        .children(
            units
                .iter()
                .enumerate()
                .map(|(index, unit)| unit_row(unit, index, index == last, entity, cx)),
        )
}

fn unit_row(
    unit: &UnitLine,
    index: usize,
    last: bool,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let inverted = unit.selected;
    let muted = if inverted {
        color::selection_muted(cx)
    } else {
        color::faint_text(cx)
    };
    let row = UnitsRow::Unit(unit.id);
    row_frame(
        SharedString::from(format!("unit-row-{index}")),
        last,
        inverted,
        row_click(entity, row),
        row_menu_click(entity, row),
        cx,
    )
    // A deactivated Unit stays listed, dimmed and tagged, rather than hidden.
    .when(unit.inactive_tag.is_some(), |this| this.opacity(0.55))
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
    .child(
        div()
            .flex_1()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(unit.name.clone())
            .children(unit.inactive_tag.clone().map(|tag| {
                div()
                    .flex_none()
                    .px(px(6.0))
                    .py(px(1.0))
                    .border_1()
                    .text_size(px(10.0))
                    .text_color(muted)
                    .child(tag)
            })),
    )
    .child(
        div()
            .w(TYPE_WIDTH)
            .flex_none()
            .text_color(muted)
            .child(unit.kind.clone()),
    )
    .child(
        div()
            .w(SOURCE_WIDTH)
            .flex_none()
            .text_color(muted)
            .child(unit.price_source.clone()),
    )
}

fn sources_table(
    columns: &SourceColumns,
    sources: &[SourceLine],
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let last = sources.len().saturating_sub(1);
    table_frame("settings-price-sources-table", cx)
        .child(
            header_row(cx)
                .child(header_cell(
                    Some(PRICE_SOURCE_NAME_WIDTH),
                    Some(&columns.name),
                ))
                .child(header_cell(None, Some(&columns.source)))
                .child(header_cell(
                    Some(PRICE_SOURCE_LAST_UPDATED_WIDTH),
                    Some(&columns.last_updated),
                )),
        )
        .children(
            sources
                .iter()
                .enumerate()
                .map(|(index, source)| source_row(source, index, index == last, entity, cx)),
        )
}

/// One Price Sources row: its name, the address it fetches from (a muted — for Manual entry) and
/// when it last did (a muted ✗ before the first fetch).
fn source_row(
    source: &SourceLine,
    index: usize,
    last: bool,
    entity: &Entity<UnitsPage>,
    cx: &App,
) -> impl IntoElement {
    let inverted = source.selected;
    let muted = if inverted {
        color::selection_muted(cx)
    } else {
        color::muted(cx)
    };
    let manual = source.kind == SourceKind::ManualEntry;
    let last_updated = if source.not_fetched {
        div()
            .flex()
            .gap(px(4.0))
            .text_color(muted)
            .child(NOT_FETCHED_GLYPH)
            .child(source.last_updated.clone())
    } else if manual {
        div().text_color(muted).child(source.last_updated.clone())
    } else {
        div()
            .text_color(if inverted {
                color::selection_muted(cx)
            } else {
                color::faint_text(cx)
            })
            .child(source.last_updated.clone())
    };
    let row = UnitsRow::Source(source.id);
    row_frame(
        SharedString::from(format!("price-source-row-{index}")),
        last,
        inverted,
        row_click(entity, row),
        row_menu_click(entity, row),
        cx,
    )
    .child(
        div()
            .w(PRICE_SOURCE_NAME_WIDTH)
            .flex_none()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(source.name.clone()),
    )
    .child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .when(manual, |this| this.text_color(muted))
            .child(source.address.clone()),
    )
    .child(last_updated.w(PRICE_SOURCE_LAST_UPDATED_WIDTH).flex_none())
}

/// The row's context menu at the pointer: `190px`, `border:2px solid`, the dialog shadow, the
/// destructive entry last in red below a 1px rule. A press anywhere outside it closes it.
fn row_menu(menu: RowMenu, at: Point<Pixels>, entity: &Entity<UnitsPage>, cx: &App) -> AnyElement {
    let hover = color::hover(cx);
    let items = menu.entries.into_iter().map(
        |MenuEntry {
             action,
             label,
             destructive,
         }| {
            let on_click = send(entity, UnitsPageEvent::Menu(action));
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
                .text_color(if destructive {
                    color::accent_text(cx)
                } else {
                    color::foreground(cx)
                })
                .hover(move |style| style.bg(hover).font_weight(gpui::FontWeight::EXTRA_BOLD))
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(label);
            if destructive {
                div()
                    .flex()
                    .flex_col()
                    .child(div().h(px(1.0)).my(px(4.0)).bg(color::hairline(cx)))
                    .child(item)
                    .into_any_element()
            } else {
                item.into_any_element()
            }
        },
    );
    let dismiss = send(entity, UnitsPageEvent::CloseMenu);
    gpui::deferred(
        gpui::anchored().position(at).snap_to_window().child(
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
                .on_mouse_down_out(move |_event, window, cx| dismiss(window, cx))
                .children(items),
        ),
    )
    .with_priority(1)
    .into_any_element()
}
