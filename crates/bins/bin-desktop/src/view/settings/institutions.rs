//! The **Institutions** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): a
//! table (INSTITUTION / ACCOUNT TYPE) seeded from `crate::settings::DEFAULT_INSTITUTIONS`,
//! per-row edit/delete buttons, and a "+ Add institution" button below it -- same shape as
//! `view::settings::units`, just a two-column table instead of three.
//!
//! "+ Add institution" opens the Add institution dialog (issue #187, not yet built), wired the
//! same way `units::add_button` stubs "+ Add unit" against its own not-yet-built dialog issue.
//! Row edit/delete have **no dialog ticket anywhere on this map** -- unlike Units, whose
//! `2c`/`2d` dialogs are named tickets (#185/#186), the README's own "Dialog lifecycle" table and
//! `State` block only ever mention `AddInstitution`, never an `EditInstitution`/
//! `DeleteInstitution` variant. So these two stubs flash a plain "not yet built" message with no
//! issue number to point at, rather than inventing one.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{settings::InstitutionRow, theme::color};

pub type OnRowIndexClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, index-less click handler -- what a [`row_action_button`] is bound to after its own
/// row index has already been curried in.
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub fn render(
    institutions: &[InstitutionRow],
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
    on_add_click: OnAddClick,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(table(institutions, on_edit_click, on_delete_click))
        .child(add_button(on_add_click))
        .into_any_element()
}

const ACCOUNT_TYPE_WIDTH: gpui::Pixels = px(140.0);
const ACTIONS_WIDTH: gpui::Pixels = px(120.0);

fn table(
    institutions: &[InstitutionRow],
    on_edit_click: OnRowIndexClick,
    on_delete_click: OnRowIndexClick,
) -> impl IntoElement {
    let last_index = institutions.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::BORDER)
        .child(table_header())
        .children(institutions.iter().enumerate().map(|(index, institution)| {
            row(
                institution,
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
        .child(div().flex_1().child("INSTITUTION"))
        .child(div().w(ACCOUNT_TYPE_WIDTH).child("ACCOUNT TYPE"))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child("ACTIONS"),
        )
}

fn row(
    institution: &InstitutionRow,
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
                .flex_1()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(institution.name),
        )
        .child(
            div()
                .w(ACCOUNT_TYPE_WIDTH)
                .text_color(color::INK_TERTIARY)
                .child(institution.account_type),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("institution-edit-{index}")),
                    "edit",
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_edit_click(index, window, cx)
                    }),
                ))
                .child(row_action_button(
                    SharedString::from(format!("institution-delete-{index}")),
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

/// The "+ Add institution" button: `padding:10px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; width:fit-content; margin-top:16px` -- the mockup's own
/// markup for this button already uses `margin-top:16px` (not the table's own inline
/// `margin-bottom:48px` slip `units::add_button`'s doc calls out), so this one needs no
/// deviation from the raw markup.
fn add_button(on_click: OnAddClick) -> impl IntoElement {
    // `align_self: flex-start` (no direct `Styled` builder for it, unlike the container-level
    // `items_start`/etc.) -- without it, this button stretches to the full width of its column
    // parent instead of shrinking to its own content, unlike `width:fit-content` in the mockup.
    let mut button = div()
        .id("settings-add-institution")
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
        .child("+ Add institution");
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
