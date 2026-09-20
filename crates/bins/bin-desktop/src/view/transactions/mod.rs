//! The **4a** Transactions page (`docs/ux/desktop/Transactions/README.md`): a full-width view --
//! the header block, the column header, then the virtualised table -- with no context rail beside
//! it (the mockup shows none, the same as Accounts and Settings).
//!
//! This module owns the page's frame and the table body (`table`); the chip row, the footer bar and
//! the filter popover are built around it by the tickets that follow, and slot into this same
//! vertical stack: fixed header, fixed column header, the one flexing list, fixed footer.
//!
//! Every row action is a callback into `Shell`, so keyboard and mouse reach the same handlers.

mod table;

pub use table::OnRowClick;

use std::rc::Rc;

use gpui::{AnyElement, Pixels, UniformListScrollHandle, div, prelude::*, px};

use crate::{theme::color, transaction_rows::RowView};

pub struct TransactionsPageProps {
    /// Every visible row, formatted per the Display preferences.
    pub rows: Rc<Vec<RowView>>,
    /// A position in `rows`, already clamped.
    pub selected: usize,
    /// The height of every row, from the Display density preference.
    pub row_height: Pixels,
    pub scroll: UniformListScrollHandle,
    pub on_row_click: OnRowClick,
}

pub fn render(focused: bool, props: TransactionsPageProps) -> AnyElement {
    div()
        .id("transactions")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .child(title_bar())
        .child(table::column_header())
        .child(table::rows(
            props.rows,
            props.selected,
            props.row_height,
            props.scroll,
            props.on_row_click,
        ))
        .into_any_element()
}

/// The view header's title row: `padding:16px 28px 14px; border-bottom:2px solid
/// rgba(32,30,29,.38)`, "Transactions" at 26px/800. The chip row and the add button join it later.
fn title_bar() -> impl IntoElement {
    div()
        .flex_none()
        .px(px(28.0))
        .pt(px(16.0))
        .pb(px(14.0))
        .border_b(px(2.0))
        .border_color(color::STRUCTURAL_RULE)
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(26.0))
                .text_color(color::INK)
                .child("Transactions"),
        )
}
