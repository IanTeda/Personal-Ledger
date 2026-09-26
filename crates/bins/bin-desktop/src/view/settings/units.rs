//! The **Units** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state", revised by
//! issue #189): a "UNITS" table title, a table (CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS)
//! seeded from `crate::settings::default_units()`, per-row edit/delete buttons, a "+ Add unit"
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

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    settings::{PriceSourceRow, UnitRow},
    theme::color,
    view::settings::SettingsSection,
};

pub type OnRowIndexClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, index-less click handler -- what a [`row_action_button`] is bound to after its own
/// row index has already been curried in.
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

#[expect(
    clippy::too_many_arguments,
    reason = "a stateless GPUI render fn takes each value and handler it wires; a props struct would only rename the list"
)]
pub fn render(
    units: &[UnitRow],
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
    on_add_click: OnAddClick,
    price_sources: &[PriceSourceRow],
    on_price_source_test_click: OnRowIndexClick,
    on_price_source_edit_click: OnRowIndexClick,
    on_price_source_delete_click: OnRowIndexClick,
    on_add_price_source_click: OnAddClick,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(subsection_label(lib_locale::format::upper(
            &SettingsSection::Units.label(),
        )))
        .child(table(units, on_edit_click, on_delete_click))
        .child(add_button(
            "settings-add-unit",
            crate::msg::desktop_settings_units_add_button("+"),
            on_add_click,
        ))
        .child(
            div()
                .mt(px(32.0))
                .flex()
                .flex_col()
                .child(subsection_label(lib_locale::format::upper(
                    &crate::msg::desktop_settings_price_sources_heading(),
                )))
                .child(price_source_table(
                    price_sources,
                    on_price_source_test_click,
                    on_price_source_edit_click,
                    on_price_source_delete_click,
                ))
                .child(add_button(
                    "settings-add-price-source",
                    crate::msg::desktop_settings_price_sources_add("+"),
                    on_add_price_source_click,
                )),
        )
        .into_any_element()
}

/// `font:800 10px/1 'Archivo'; letter-spacing:.11em; color:#9b9797; margin-bottom:10px` -- the
/// same treatment "PRICE SOURCES" and General's own "THIS LEDGER" label use.
fn subsection_label(text: String) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .mb(px(10.0))
        .child(text)
}

const CODE_WIDTH: gpui::Pixels = px(100.0);
const NAME_WIDTH: gpui::Pixels = px(180.0);
const SOURCE_WIDTH: gpui::Pixels = px(130.0);
const TYPE_WIDTH: gpui::Pixels = px(80.0);
const ACTIONS_WIDTH: gpui::Pixels = px(120.0);

fn table(
    units: &[UnitRow],
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
) -> impl IntoElement {
    let last_index = units.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::BORDER)
        .child(table_header())
        .children(units.iter().enumerate().map(|(index, unit)| {
            row(
                unit,
                index == last_index,
                index,
                on_edit_click.clone(),
                on_delete_click.clone(),
            )
        }))
}

fn table_header() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .bg(color::CHROME)
        .border_b(px(1.0))
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_SECONDARY)
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
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(lib_locale::format::upper(&lib_locale::msg::column_actions())),
        )
}

fn row(
    unit: &UnitRow,
    last: bool,
    index: usize,
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
        })
        .child(
            div()
                .w(CODE_WIDTH)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(unit.code.clone()),
        )
        .child(div().w(NAME_WIDTH).child(unit.name.clone()))
        .child(flags_cell(unit))
        .child(
            div()
                .w(SOURCE_WIDTH)
                .text_color(color::INK_TERTIARY)
                .child(unit.source.clone()),
        )
        .child(
            div()
                .w(TYPE_WIDTH)
                .text_color(color::INK_TERTIARY)
                .child(unit.kind.clone()),
        )
        .child(
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
                ))
                .child(row_action_button(
                    SharedString::from(format!("unit-delete-{index}")),
                    crate::msg::desktop_hint_delete(),
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(index, window, cx)
                    }),
                )),
        )
}

/// The FLAGS cell: `base`/`default` tag pills, left-aligned next to NAME (NAME is a fixed
/// 180px column, not flex, so flags never drift into empty middle space) -- empty for every row
/// but the ledger's own base/default unit.
fn flags_cell(unit: &UnitRow) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .gap(px(6.0))
        .justify_start()
        .when(unit.is_base, |this| {
            this.child(tag_accent(crate::msg::desktop_settings_units_flag_base()))
        })
        .when(unit.is_default, |this| {
            this.child(tag_outline(
                crate::msg::desktop_settings_units_flag_default(),
            ))
        })
}

/// `.tag.tag-accent`: `background: var(--color-accent-100); color: var(--color-accent-800)`.
/// Square corners, not the shared design system's own rounded pill -- this crate's "Radius 0
/// everywhere" rule (`theme::color::TAG_ACCENT_BG`'s own doc) has no exception for a general tag
/// shape.
fn tag_accent(label: String) -> impl IntoElement {
    div()
        .bg(color::TAG_ACCENT_BG)
        .text_color(color::TAG_ACCENT_TEXT)
        .text_size(px(11.0))
        .px(px(10.0))
        .py(px(3.0))
        .child(label)
}

/// `.tag.tag-outline`: `border: 1px solid var(--color-accent); color: var(--color-accent)`.
fn tag_outline(label: String) -> impl IntoElement {
    div()
        .border_1()
        .border_color(color::ACCENT)
        .text_color(color::ACCENT)
        .text_size(px(11.0))
        .px(px(10.0))
        .py(px(3.0))
        .child(label)
}

const PRICE_SOURCE_NAME_WIDTH: gpui::Pixels = px(130.0);
const PRICE_SOURCE_LAST_UPDATED_WIDTH: gpui::Pixels = px(130.0);
const PRICE_SOURCE_ACTIONS_WIDTH: gpui::Pixels = px(170.0);

fn price_source_table(
    rows: &[PriceSourceRow],
    on_test_click: OnRowIndexClick,
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
) -> impl IntoElement {
    let last_index = rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(16.0))
        .border_1()
        .border_color(color::BORDER)
        .child(price_source_table_header())
        .children(rows.iter().enumerate().map(|(index, source)| {
            price_source_row(
                source,
                index == last_index,
                index,
                on_test_click.clone(),
                on_edit_click.clone(),
                on_delete_click.clone(),
            )
        }))
}

fn price_source_table_header() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .bg(color::CHROME)
        .border_b(px(1.0))
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_SECONDARY)
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
        .child(
            div()
                .w(PRICE_SOURCE_ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(lib_locale::format::upper(&lib_locale::msg::column_actions())),
        )
}

/// One Price Sources row -- keyed by the unit's own `name`, not `code` (the README's own "the
/// one place the unit is referenced by name rather than code" callout, since this table is
/// about the relationship to an external source, not the unit record itself).
fn price_source_row(
    source: &PriceSourceRow,
    last: bool,
    index: usize,
    on_test_click: OnRowIndexClick,
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
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
                .text_color(color::INK_TERTIARY)
                .child(source.last_updated.clone()),
        )
        .child(
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
                ))
                .child(row_action_button(
                    SharedString::from(format!("price-source-edit-{index}")),
                    crate::msg::desktop_hint_edit(),
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_edit_click(index, window, cx)
                    }),
                ))
                .child(row_action_button(
                    SharedString::from(format!("price-source-delete-{index}")),
                    crate::msg::desktop_hint_delete(),
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(index, window, cx)
                    }),
                )),
        )
}

/// A row action button: `padding:4px 10px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-size:11px`.
fn row_action_button(id: SharedString, label: String, on_click: OnPlainClick) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::BORDER)
        .text_size(px(11.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

/// A section button: `padding:10px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; width:fit-content` -- see the module doc for why this
/// sits `mt(16px)` below its own table rather than replicating the table's own inline
/// `margin-bottom:48px`.
fn add_button(id: &'static str, label: String, on_click: OnAddClick) -> impl IntoElement {
    // `align_self: flex-start` (no direct `Styled` builder for it, unlike the container-level
    // `items_start`/etc.) -- without it, this button stretches to the full width of its column
    // parent instead of shrinking to its own content, unlike `width:fit-content` in the mockup.
    let mut button = div()
        .id(id)
        .cursor_pointer()
        .mt(px(16.0))
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::CHROME)
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label);
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
