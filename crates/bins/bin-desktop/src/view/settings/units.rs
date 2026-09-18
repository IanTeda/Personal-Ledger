//! The **Units** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): a table
//! (CODE / NAME / TYPE) seeded from `crate::settings::DEFAULT_UNITS`, per-row edit/delete
//! buttons, and a "+ Add unit" button below it.
//!
//! Row edit/delete and "+ Add unit" all open a dialog this map hasn't built yet (issues
//! #184-#186, blocked on this ticket) -- each click is wired to a clearly-marked stub instead
//! (`Shell::handle_unit_edit_click`/`handle_unit_delete_click`/`handle_add_unit_click`, which
//! flash a "not yet built" status-line message naming the ticket), per this ticket's own body:
//! "if the dialog tickets land first, wire the real open call, otherwise leave a clearly-marked
//! stub call this ticket's own follow-on fills in."
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

use crate::{settings::UnitRow, theme::color};

pub type OnRowIndexClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, index-less click handler -- what a [`row_action_button`] is bound to after its own
/// row index has already been curried in.
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub fn render(
    units: &[UnitRow],
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
    on_add_click: OnAddClick,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(table(units, on_edit_click, on_delete_click))
        .child(add_button(on_add_click))
        .into_any_element()
}

const CODE_WIDTH: gpui::Pixels = px(100.0);
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
        .child(div().w(CODE_WIDTH).child("CODE"))
        .child(div().flex_1().child("NAME"))
        .child(div().w(TYPE_WIDTH).child("TYPE"))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child("ACTIONS"),
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
                .child(unit.code),
        )
        .child(div().flex_1().child(unit.name))
        .child(
            div()
                .w(TYPE_WIDTH)
                .text_color(color::INK_TERTIARY)
                .child(unit.kind),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("unit-edit-{index}")),
                    "edit",
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_edit_click(index, window, cx)
                    }),
                ))
                .child(row_action_button(
                    SharedString::from(format!("unit-delete-{index}")),
                    "delete",
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(index, window, cx)
                    }),
                )),
        )
}

/// A row action button: `padding:4px 10px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-size:11px`.
fn row_action_button(
    id: SharedString,
    label: &'static str,
    on_click: OnPlainClick,
) -> impl IntoElement {
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

/// The "+ Add unit" button: `padding:10px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; width:fit-content` -- see the module doc for why this
/// sits `mt(16px)` below the table rather than replicating the table's own inline
/// `margin-bottom:48px`.
fn add_button(on_click: OnAddClick) -> impl IntoElement {
    // `align_self: flex-start` (no direct `Styled` builder for it, unlike the container-level
    // `items_start`/etc.) -- without it, this button stretches to the full width of its column
    // parent instead of shrinking to its own content, unlike `width:fit-content` in the mockup.
    let mut button = div()
        .id("settings-add-unit")
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
        .child("+ Add unit");
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
