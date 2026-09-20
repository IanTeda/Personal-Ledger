//! The **4a** Transactions page (`docs/ux/desktop/Transactions/README.md`): a full-width view --
//! the header block, the column header, then the virtualised table -- with no context rail beside
//! it (the mockup shows none, the same as Accounts and Settings).
//!
//! This module owns the page's frame: the `header` (title, count line, add button, chip row,
//! search), the `table` (column header and virtualised rows) and the `footer`, stacked as a fixed
//! header, a fixed column header, the one flexing list and a fixed footer. The filter popover is
//! built over it by the next ticket.
//!
//! Every row action is a callback into `Shell`, so keyboard and mouse reach the same handlers.

mod filter_popover;
mod footer;
mod header;
mod table;

pub use filter_popover::{OnFieldClick, OnOptionClick, OnStatusClick, PopoverProps};
pub use header::{ChipBounds, HeaderProps, OnChipClick, OnPlainClick};
pub use table::OnRowClick;

use std::rc::Rc;

use gpui::{AnyElement, Pixels, UniformListScrollHandle, div, prelude::*, px};

use crate::{theme::color, transaction_chips::Footer, transaction_rows::RowView};

pub struct TransactionsPageProps {
    /// The filter popover is open: the page dims to 55% (no blur in gpui 0.2, so opacity only).
    pub dimmed: bool,
    pub header: HeaderProps,
    /// Every visible row, formatted per the Display preferences.
    pub rows: Rc<Vec<RowView>>,
    /// A position in `rows`, already clamped.
    pub selected: usize,
    /// The height of every row, from the Display density preference.
    pub row_height: Pixels,
    pub scroll: UniformListScrollHandle,
    pub on_row_click: OnRowClick,
    pub footer: Footer,
}

/// The page: the fixed header block, the fixed column header, the one flexing list, the fixed footer.
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
        .when(props.dimmed, |this| this.opacity(0.55))
        .child(header::render(props.header))
        .child(table::column_header())
        .child(table::rows(
            props.rows,
            props.selected,
            props.row_height,
            props.scroll,
            props.on_row_click,
        ))
        .child(footer::render(&props.footer))
        .into_any_element()
}

/// Draws the filter popover over the window: see [`filter_popover`].
pub fn render_popover(props: PopoverProps<'_>) -> AnyElement {
    filter_popover::render(props)
}
