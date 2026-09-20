//! The 4a table itself (`docs/ux/desktop/Transactions/README.md`): the column header and the
//! virtualised rows. Rows are `gpui::uniform_list` items (the research ticket's recommendation:
//! hand-rolled, not `gpui_component::Table`, because the row height follows the Display density
//! preference and the keyboard model is the shell's own), so only the visible ones are built.
//!
//! **One width table serves the header and every row** (`STATUS_WIDTH` and its neighbours), because
//! the column header sits outside the scrolling list and any drift would misalign the columns. The
//! PAYEE column is the flexible one and has a floor, so a narrow window clips the fixed columns
//! rather than squeezing the payee to nothing (the same lesson the Accounts page learned).
//!
//! The selected row is the dark treatment: ink fill, ground text, secondary text stepped down to
//! `#d7d3d3`, payee at weight 800, negatives in the on-dark accent.

use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Pixels, SharedString, UniformListScrollHandle, Window, div, prelude::*, px,
    uniform_list,
};

use crate::{
    theme::color,
    transaction_rows::{RowView, TagsCell},
};

pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

const STATUS_WIDTH: Pixels = px(20.0);
const FLAG_WIDTH: Pixels = px(16.0);
/// Wide enough for the longest compact date, `28 jan 2025`.
const DATE_WIDTH: Pixels = px(92.0);
const ACCOUNT_WIDTH: Pixels = px(116.0);
const PAYEE_MIN_WIDTH: Pixels = px(120.0);
const CATEGORY_WIDTH: Pixels = px(100.0);
const TAGS_WIDTH: Pixels = px(96.0);
const AMOUNT_WIDTH: Pixels = px(96.0);
const RUNNING_WIDTH: Pixels = px(104.0);

/// Horizontal padding of the header and every row: the bundle's `28px`.
const PAGE_PADDING_X: Pixels = px(28.0);

/// `padding:8px 28px; 10px/800; #9b9797; border-bottom:1px solid #d7d3d3`.
pub fn column_header() -> impl IntoElement {
    let head = |width: Pixels, label: &'static str, right: bool| {
        let cell = div().w(width).flex_none().truncate().child(label);
        if right {
            cell.text_align(gpui::TextAlign::Right)
        } else {
            cell
        }
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(PAGE_PADDING_X)
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(color::HAIRLINE)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .child(div().w(STATUS_WIDTH).flex_none())
        .child(div().w(FLAG_WIDTH).flex_none())
        .child(head(DATE_WIDTH, "DATE", false))
        .child(head(ACCOUNT_WIDTH, "ACCOUNT", false))
        .child(div().flex_1().min_w(PAYEE_MIN_WIDTH).child("PAYEE"))
        .child(head(CATEGORY_WIDTH, "CATEGORY", false))
        .child(head(TAGS_WIDTH, "TAGS", false))
        .child(head(AMOUNT_WIDTH, "AMOUNT", true))
        .child(head(RUNNING_WIDTH, "RUNNING", true))
}

/// The scrolling body: a `uniform_list` of `rows`, `selected` drawn dark, every row `row_height`
/// tall. `flex_1` and a zero minimum height let it take exactly the space between the fixed header
/// above and whatever sits below. An empty result shows a message instead of an empty list.
pub fn rows(
    rows: Rc<Vec<RowView>>,
    selected: usize,
    row_height: Pixels,
    scroll: UniformListScrollHandle,
    on_row_click: OnRowClick,
) -> AnyElement {
    if rows.is_empty() {
        return div()
            .flex_1()
            .min_h(px(0.0))
            .px(PAGE_PADDING_X)
            .py(px(24.0))
            .text_color(color::INK_SECONDARY)
            .child("No transactions match these filters.")
            .into_any_element();
    }

    let count = rows.len();
    uniform_list(
        "transactions-rows",
        count,
        // Called for the visible range, and once for row 0 to measure: a pure function of the
        // index, with every cell already formatted (see `transaction_rows::build_rows`).
        move |range: Range<usize>, _window: &mut Window, _cx: &mut App| {
            range
                .map(|index| {
                    row(
                        index,
                        &rows[index],
                        index == selected,
                        row_height,
                        on_row_click.clone(),
                    )
                })
                .collect::<Vec<_>>()
        },
    )
    .track_scroll(scroll)
    .flex_1()
    .min_h(px(0.0))
    .into_any_element()
}

fn row(
    index: usize,
    view: &RowView,
    selected: bool,
    height: Pixels,
    on_click: OnRowClick,
) -> AnyElement {
    let (primary, secondary, tertiary) = if selected {
        (
            color::INK_ON_DARK,
            color::INK_ON_DARK_TERTIARY,
            color::INK_TERTIARY,
        )
    } else {
        (color::INK, color::INK_SECONDARY, color::INK_TERTIARY)
    };
    let negative = if selected {
        color::ACCENT_ON_DARK
    } else {
        color::ACCENT_TEXT
    };
    let payee_weight = if selected {
        gpui::FontWeight::EXTRA_BOLD
    } else {
        gpui::FontWeight::NORMAL
    };

    div()
        .id(("transactions-row", index))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .w_full()
        .h(height)
        .px(PAGE_PADDING_X)
        .border_b(px(1.0))
        .border_color(if selected {
            color::INK
        } else {
            color::HAIRLINE_LIGHT
        })
        .when(selected, |this| this.bg(color::INK).text_color(primary))
        .when(!selected, |this| {
            this.hover(|style| style.bg(color::HOVER_TINT))
        })
        .on_click(move |_event, window, cx| on_click(index, window, cx))
        .child(
            div()
                .w(STATUS_WIDTH)
                .flex_none()
                .text_color(primary)
                .child(view.status_glyph),
        )
        .child(
            div()
                .w(FLAG_WIDTH)
                .flex_none()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(primary)
                .children(view.flag_glyph),
        )
        .child(
            div()
                .w(DATE_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(view.date.clone()),
        )
        .child(
            div()
                .w(ACCOUNT_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(view.account.clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w(PAYEE_MIN_WIDTH)
                .truncate()
                .font_weight(payee_weight)
                .text_color(primary)
                .child(view.payee.clone()),
        )
        .child(
            div()
                .w(CATEGORY_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(view.category.clone()),
        )
        .child(tags_cell(&view.tags, selected, tertiary, secondary))
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .text_color(if view.amount_negative {
                    negative
                } else {
                    primary
                })
                .child(view.amount.clone()),
        )
        .child(
            div()
                .w(RUNNING_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .text_color(if view.running_negative {
                    negative
                } else {
                    primary
                })
                .children(view.running.clone().map(SharedString::from)),
        )
        .into_any_element()
}

/// `—` when untagged, else the first tag as a neutral chip (`padding:1px 6px; 10px`) with `+N`
/// after it for the rest.
fn tags_cell(
    tags: &TagsCell,
    selected: bool,
    tertiary: gpui::Rgba,
    secondary: gpui::Rgba,
) -> impl IntoElement {
    let cell = div()
        .w(TAGS_WIDTH)
        .flex_none()
        .flex()
        .items_center()
        .gap(px(4.0));
    match tags {
        TagsCell::None => cell
            .text_color(tertiary)
            .child(crate::transaction_rows::EMPTY_CELL),
        TagsCell::Chips { first, more } => {
            let chip = div()
                .min_w(px(0.0))
                .truncate()
                .px(px(6.0))
                .py(px(1.0))
                .text_size(px(10.0))
                .border_1()
                .when(selected, |this| {
                    this.border_color(color::INK_ON_DARK_SECONDARY)
                        .text_color(color::INK_ON_DARK_TERTIARY)
                })
                .when(!selected, |this| {
                    this.bg(color::CHROME)
                        .border_color(color::BORDER)
                        .text_color(color::INK_SECONDARY)
                })
                .child(first.clone());
            cell.child(chip).when(*more > 0, |this| {
                this.child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .text_color(secondary)
                        .child(format!("+{more}")),
                )
            })
        }
    }
}
