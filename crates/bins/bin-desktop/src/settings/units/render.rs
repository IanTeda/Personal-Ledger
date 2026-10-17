//! Draws the **Units** page (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state",
//! revised by issue #189): a "UNITS" table title, a table (CODE / NAME / FLAGS / SOURCE / TYPE /
//! ACTIONS) read from the Ledger, per-row edit/delete buttons, a "+ Add unit"
//! button, and (new in #189) a **Price Sources** subsection with its own NAME/SOURCE/LAST
//! UPDATED/ACTIONS table and "+ Add price source" button. This section absorbed the removed
//! "Ledger & units" section's own "Default unit for new entries" control, now the FLAGS
//! column's `default` pill on whichever row carries it, rather than a separate standalone
//! control.
//!
//! Row edit/delete/"+ Add unit" open the real Add/Edit/Delete unit dialogs (issues #184-#186).
//! Price Sources' own test/edit/delete/"+ Add price source" have **no dialog ticket anywhere on
//! this map** -- same reasoning as Institutions' own row edit/delete (issue #178's own doc):
//! nothing in the README's "Dialog lifecycle" table names one, so these flash a plain "not yet
//! built" status message with no issue number to point at.
//!
//! The table's own outer `margin-bottom:48px` in the raw mockup markup is **not** replicated --
//! it would put a 48px gap between the table and its own "+ Add unit" button, contradicting the
//! README's own implementation note 4 ("Section gap is 48px, uniformly... carried on a single
//! edge") and the Institutions section's own markup a few lines later, which uses a plain
//! `margin-top:16px` on its "+ Add institution" button instead. Treated as a mockup-authoring
//! slip, not the intended spacing -- built to the 16px gap that's consistent both with
//! Institutions' own button and with the stated rule.

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Entity, SharedString, Window, div, prelude::*, px};
use lib_locale::Label;
use lib_units::{PriceSourceRow, UnitId, UnitRow};

use super::UnitsPage;
use crate::{settings::SettingsPage, theme::color};

type OnRowIndexClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, index-less click handler -- what a [`row_action_button`] is bound to after its own
/// row index has already been curried in.
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

impl Render for UnitsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let docked = self.docked;
        let selected = if docked { self.selected_id(cx) } else { None };
        let cx: &App = cx;
        div()
            .flex()
            .flex_col()
            .child(subsection_label(
                lib_locale::format::upper(&SettingsPage::Units.label()),
                cx,
            ))
            .child(table(
                self.units(cx),
                UnitHandlers {
                    select: on_row(&entity, UnitsPage::click_select),
                    edit: on_row(&entity, UnitsPage::click_edit),
                    delete: on_row(&entity, UnitsPage::click_delete),
                },
                docked,
                selected,
                cx,
            ))
            .child(add_button(
                "settings-add-unit",
                crate::msg::desktop_settings_units_add_button("+"),
                on_plain(&entity, UnitsPage::click_add),
                cx,
            ))
            .child(
                div()
                    .mt(px(32.0))
                    .flex()
                    .flex_col()
                    .child(subsection_label(
                        lib_locale::format::upper(
                            &crate::msg::desktop_settings_price_sources_heading(),
                        ),
                        cx,
                    ))
                    .child(price_source_table(
                        self.price_sources(cx),
                        docked,
                        SourceHandlers {
                            test: row_stub(
                                &entity,
                                crate::msg::desktop_status_test_price_source_not_yet_built,
                            ),
                            edit: row_stub(
                                &entity,
                                crate::msg::desktop_status_edit_price_source_not_yet_built,
                            ),
                            delete: row_stub(
                                &entity,
                                crate::msg::desktop_status_delete_price_source_not_yet_built,
                            ),
                        },
                        cx,
                    ))
                    .child(add_button(
                        "settings-add-price-source",
                        crate::msg::desktop_settings_price_sources_add("+"),
                        stub(
                            &entity,
                            crate::msg::desktop_status_add_price_source_not_yet_built,
                        ),
                        cx,
                    )),
            )
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
) -> OnAddClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, act))
}

/// As [`on_plain`], for a button with nothing behind it yet: it says so on the status line.
fn stub(entity: &Entity<UnitsPage>, message: fn() -> String) -> OnAddClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, |page, cx| page.not_built(message(), cx)))
}

/// As [`stub`], for a row's button, whose row it ignores.
fn row_stub(entity: &Entity<UnitsPage>, message: fn() -> String) -> OnRowIndexClick {
    let entity = entity.clone();
    Rc::new(move |_index, _window, cx| entity.update(cx, |page, cx| page.not_built(message(), cx)))
}

/// `font:800 10px/1 'Archivo'; letter-spacing:.11em; color:#9b9797; margin-bottom:10px` -- the
/// same treatment "PRICE SOURCES" and General's own "THIS LEDGER" label use.
fn subsection_label(text: String, cx: &App) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .mb(px(10.0))
        .child(text)
}

const CODE_WIDTH: gpui::Pixels = px(100.0);
const NAME_WIDTH: gpui::Pixels = px(180.0);
const SOURCE_WIDTH: gpui::Pixels = px(180.0);
const TYPE_WIDTH: gpui::Pixels = px(80.0);
const ACTIONS_WIDTH: gpui::Pixels = px(120.0);

/// What a Units row's clicks do: select it (X-Large), or open its Edit or Delete Dialog (the
/// ACTIONS buttons, which X-Large drops for the detail pane).
#[derive(Clone)]
struct UnitHandlers {
    select: OnRowIndexClick,
    edit: OnRowIndexClick,
    delete: OnRowIndexClick,
}

/// Where a Units row sits and how it is drawn.
#[derive(Clone, Copy)]
struct RowState {
    index: usize,
    last: bool,
    /// Whether the detail pane is docked: the ACTIONS column is dropped and rows are selectable.
    docked: bool,
    selected: bool,
}

fn table(
    units: &[UnitRow],
    handlers: UnitHandlers,
    docked: bool,
    selected: Option<UnitId>,
    cx: &App,
) -> impl IntoElement {
    let last_index = units.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(docked, cx))
        .children(units.iter().enumerate().map(|(index, unit)| {
            let state = RowState {
                index,
                last: index == last_index,
                docked,
                selected: selected == Some(unit.id),
            };
            row(unit, state, handlers.clone(), cx)
        }))
}

fn table_header(docked: bool, cx: &App) -> impl IntoElement {
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
        .child(
            div()
                .w(CODE_WIDTH)
                .child(lib_locale::format::upper(&lib_locale::msg::column_code())),
        )
        .child(
            div()
                .w(NAME_WIDTH)
                .child(lib_locale::format::upper(&lib_locale::msg::column_name())),
        )
        .child(div().flex_1().child(lib_locale::format::upper(
            &crate::msg::desktop_settings_units_column_flags(),
        )))
        .child(div().w(SOURCE_WIDTH).child(lib_locale::format::upper(
            &crate::msg::desktop_settings_units_column_source(),
        )))
        .child(
            div()
                .w(TYPE_WIDTH)
                .child(lib_locale::format::upper(&lib_locale::msg::column_type())),
        )
        .when(!docked, |this| {
            this.child(
                div()
                    .w(ACTIONS_WIDTH)
                    .text_align(gpui::TextAlign::Right)
                    .child(lib_locale::format::upper(&lib_locale::msg::column_actions())),
            )
        })
}

fn row(unit: &UnitRow, state: RowState, handlers: UnitHandlers, cx: &App) -> impl IntoElement {
    let RowState {
        index,
        last,
        docked,
        selected,
    } = state;
    let muted = if selected {
        color::selection_muted(cx)
    } else {
        color::faint_text(cx)
    };
    let on_select = handlers.select;
    let on_edit_click = handlers.edit;
    let on_delete_click = handlers.delete;
    div()
        .debug_selector(move || format!("unit-row-{index}"))
        .id(SharedString::from(format!("unit-row-{index}")))
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(docked, |this| {
            this.cursor_pointer()
                .on_click(move |_event, window, cx| on_select(index, window, cx))
        })
        .child(
            div()
                .w(CODE_WIDTH)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(unit.code.clone()),
        )
        .child(div().w(NAME_WIDTH).child(unit.name.clone()))
        .child(flags_cell(unit, cx))
        .child(
            div()
                .w(SOURCE_WIDTH)
                .text_color(muted)
                .child(unit.source.clone()),
        )
        .child(
            div()
                .w(TYPE_WIDTH)
                .text_color(muted)
                .child(unit.kind.label()),
        )
        .when(!docked, |this| {
            this.child(
                div()
                    .w(ACTIONS_WIDTH)
                    .flex()
                    .justify_end()
                    .gap(px(10.0))
                    .child(row_action_button(
                        SharedString::from(format!("unit-edit-{index}")),
                        crate::msg::desktop_hint_edit(),
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_edit_click(index, window, cx)
                        }),
                        cx,
                    ))
                    .child(row_action_button(
                        SharedString::from(format!("unit-delete-{index}")),
                        crate::msg::desktop_hint_delete(),
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_delete_click(index, window, cx)
                        }),
                        cx,
                    )),
            )
        })
}

/// The FLAGS cell: `base`/`default` tag pills, left-aligned next to NAME (NAME is a fixed
/// 180px column, not flex, so flags never drift into empty middle space) -- empty for every row
/// but the ledger's own base/default unit.
fn flags_cell(unit: &UnitRow, cx: &App) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .gap(px(6.0))
        .justify_start()
        .when(unit.is_base, |this| {
            this.child(tag_accent(
                crate::msg::desktop_settings_units_flag_base(),
                cx,
            ))
        })
        .when(unit.is_default, |this| {
            this.child(tag_outline(
                crate::msg::desktop_settings_units_flag_default(),
                cx,
            ))
        })
}

/// `.tag.tag-accent`: `background: var(--color-accent-100); color: var(--color-accent-800)`.
/// Square corners, not the shared design system's own rounded pill -- this crate's "Radius 0
/// everywhere" rule (`theme::color::accent_tint`'s own doc) has no exception for a general tag
/// shape.
fn tag_accent(label: String, cx: &App) -> impl IntoElement {
    div()
        .bg(color::accent_tint(cx))
        .text_color(color::accent_tint_text(cx))
        .text_size(px(11.0))
        .px(px(10.0))
        .py(px(3.0))
        .child(label)
}

/// `.tag.tag-outline`: `border: 1px solid var(--color-accent); color: var(--color-accent)`.
fn tag_outline(label: String, cx: &App) -> impl IntoElement {
    div()
        .border_1()
        .border_color(color::accent(cx))
        .text_color(color::accent(cx))
        .text_size(px(11.0))
        .px(px(10.0))
        .py(px(3.0))
        .child(label)
}

const PRICE_SOURCE_NAME_WIDTH: gpui::Pixels = px(200.0);
const PRICE_SOURCE_LAST_UPDATED_WIDTH: gpui::Pixels = px(130.0);
const PRICE_SOURCE_ACTIONS_WIDTH: gpui::Pixels = px(170.0);

/// What a Price Sources row's buttons do (all not yet built).
#[derive(Clone)]
struct SourceHandlers {
    test: OnRowIndexClick,
    edit: OnRowIndexClick,
    delete: OnRowIndexClick,
}

fn price_source_table(
    rows: &[PriceSourceRow],
    docked: bool,
    handlers: SourceHandlers,
    cx: &App,
) -> impl IntoElement {
    let last_index = rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .child(price_source_table_header(docked, cx))
        .children(rows.iter().enumerate().map(|(index, source)| {
            price_source_row(
                source,
                index == last_index,
                docked,
                index,
                handlers.clone(),
                cx,
            )
        }))
}

fn price_source_table_header(docked: bool, cx: &App) -> impl IntoElement {
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
        .child(
            div()
                .w(PRICE_SOURCE_NAME_WIDTH)
                .child(lib_locale::format::upper(&lib_locale::msg::column_name())),
        )
        .child(div().flex_1().child(lib_locale::format::upper(
            &crate::msg::desktop_settings_units_column_source(),
        )))
        .child(
            div()
                .w(PRICE_SOURCE_LAST_UPDATED_WIDTH)
                .child(lib_locale::format::upper(
                    &crate::msg::desktop_settings_units_column_last_updated(),
                )),
        )
        .when(!docked, |this| {
            this.child(
                div()
                    .w(PRICE_SOURCE_ACTIONS_WIDTH)
                    .text_align(gpui::TextAlign::Right)
                    .child(lib_locale::format::upper(&lib_locale::msg::column_actions())),
            )
        })
}

/// One Price Sources row -- keyed by the unit's own `name`, not `code` (the README's own "the
/// one place the unit is referenced by name rather than code" callout, since this table is
/// about the relationship to an external source, not the unit record itself).
fn price_source_row(
    source: &PriceSourceRow,
    last: bool,
    docked: bool,
    index: usize,
    handlers: SourceHandlers,
    cx: &App,
) -> impl IntoElement {
    let SourceHandlers {
        test: on_test_click,
        edit: on_edit_click,
        delete: on_delete_click,
    } = handlers;
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(
            div()
                .w(PRICE_SOURCE_NAME_WIDTH)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(source.unit_name.clone()),
        )
        .child(div().flex_1().child(source.source.clone()))
        .child(
            div()
                .w(PRICE_SOURCE_LAST_UPDATED_WIDTH)
                .text_color(color::faint_text(cx))
                .child(source.last_updated.clone()),
        )
        .when(!docked, |this| {
            this.child(
                div()
                    .w(PRICE_SOURCE_ACTIONS_WIDTH)
                    .flex()
                    .justify_end()
                    .gap(px(10.0))
                    .child(row_action_button(
                        SharedString::from(format!("price-source-test-{index}")),
                        crate::msg::desktop_settings_units_test(),
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_test_click(index, window, cx)
                        }),
                        cx,
                    ))
                    .child(row_action_button(
                        SharedString::from(format!("price-source-edit-{index}")),
                        crate::msg::desktop_hint_edit(),
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_edit_click(index, window, cx)
                        }),
                        cx,
                    ))
                    .child(row_action_button(
                        SharedString::from(format!("price-source-delete-{index}")),
                        crate::msg::desktop_hint_delete(),
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_delete_click(index, window, cx)
                        }),
                        cx,
                    )),
            )
        })
}

/// A row action button: `padding:4px 10px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-size:11px`.
fn row_action_button(
    id: SharedString,
    label: String,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(11.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

/// A section button: `padding:10px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; width:fit-content` -- see the module doc for why this
/// sits `mt(16px)` below its own table rather than replicating the table's own inline
/// `margin-bottom:48px`.
fn add_button(id: &'static str, label: String, on_click: OnAddClick, cx: &App) -> impl IntoElement {
    // `align_self: flex-start` (no direct `Styled` builder for it, unlike the container-level
    // `items_start`/etc.) -- without it, this button stretches to the full width of its column
    // parent instead of shrinking to its own content, unlike `width:fit-content` in the mockup.
    let mut button = div()
        .debug_selector(move || id.to_string())
        .id(id)
        .cursor_pointer()
        .mt(px(16.0))
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label);
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}

// ---------------------------------------------------------------------------------------------
// The X-Large detail pane (DESKTOP-125)
// ---------------------------------------------------------------------------------------------

/// `width:400px` at the View's right edge.
const PANE_WIDTH: gpui::Pixels = px(400.0);
/// Every pane button is 112 x 34px, so three fill the 368px row with 8px gaps.
const PANE_BUTTON_WIDTH: gpui::Pixels = px(112.0);
const PANE_BUTTON_HEIGHT: gpui::Pixels = px(34.0);

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
