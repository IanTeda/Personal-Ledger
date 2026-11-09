//! The Transactions page's Views (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a): a
//! full-width view -- the header block, the column header, then the virtualised table -- with no
//! context rail beside it (the mockup shows none, the same as Accounts and Settings).
//!
//! `page.rs` is the page Presenter's `PageView`, which the generic `ModulePage` hosts as the
//! Transactions destination; this module draws its view model: the `header` (title, count line, add button,
//! chip row, search), the `table` (column header and virtualised rows) and the `footer`, stacked as
//! a fixed header, a fixed column header, the one flexing list and a fixed footer, with the filter
//! popover drawn over the window while it is open. The Import step shows in its place while it is
//! open (`crate::transaction_import`). Each part draws a borrow of the view model and sends its
//! clicks as the Presenter's events through [`PageClicks`], so a click and a key reach the
//! Presenter alike.

mod filter_popover;
mod footer;
mod header;
pub(crate) mod hints;
pub mod page;
pub mod snapshot;
pub(crate) mod table;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Window, div, point, prelude::*, px};

use lib_transactions::presenter::{ChipField, FilterViewModel, TransactionsPageViewModel};

use crate::{shell::view::page_clicks::PageClicks, view::format};
use page::{TransactionsPage, TransactionsWidgets};

/// The page from `view`: the header block, the column header, the one flexing list and the
/// footer, with the filter popover over the window while it is open.
pub(super) fn render(
    mut view: TransactionsPageViewModel,
    widgets: &mut TransactionsWidgets,
    clicks: &PageClicks<TransactionsPage>,
    window: &mut Window,
    cx: &mut Context<'_, TransactionsPage>,
) -> AnyElement {
    let popover = view
        .filter
        .as_ref()
        .map(|filter| popover(filter, widgets, window, clicks, cx));
    let cx: &App = cx;
    let header = header::render(&view, &widgets.chip_bounds, clicks, cx);
    let row_height = px(format::row_height_px(view.density));
    // The rows move into the virtualised list, whose builder outlives this borrow.
    let rows = Rc::new(std::mem::take(&mut view.rows));
    // The page: the fixed header block, the fixed column header, the one flexing list, the
    // fixed footer. While the filter popover is open it dims to 55% (no blur in gpui 0.2, so
    // opacity only).
    let page = div()
        .id("transactions")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .when(view.filter.is_some(), |this| this.opacity(0.55))
        .child(header)
        .child(table::column_header(cx))
        .child(table::rows(
            rows,
            view.selected,
            row_height,
            widgets.scroll.clone(),
            clicks,
            cx,
        ))
        .child(footer::render(&view.footer, cx));
    div()
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .child(page)
        .children(popover)
        .into_any_element()
}

/// The filter popover, drawn over the whole window (its cancelling layer covers the rails too)
/// and anchored just below the chip that opened it, left-aligned to it and kept inside the
/// window; before any chip has been painted, at a fixed spot.
fn popover(
    view: &FilterViewModel,
    widgets: &mut TransactionsWidgets,
    window: &Window,
    clicks: &PageClicks<TransactionsPage>,
    cx: &App,
) -> AnyElement {
    let viewport = window.viewport_size();
    let field = widgets.filter_anchor.unwrap_or(ChipField::Account);
    let anchor = widgets.chip_bounds.borrow().get(&field).copied();
    let (left, top) = match anchor {
        Some(bounds) => {
            let left = bounds.origin.x.min(viewport.width - px(416.0)).max(px(8.0));
            (left, bounds.origin.y + bounds.size.height + px(6.0))
        }
        None => (px(300.0), px(200.0)),
    };
    let card = filter_popover::render(
        view,
        &mut widgets.select_window,
        point(left, top),
        clicks,
        cx,
    );
    // A window-sized layer at the window's origin, painted after everything else, so the card's
    // window coordinates hold wherever the page itself sits.
    gpui::deferred(
        gpui::anchored()
            .position(point(px(0.0), px(0.0)))
            .child(div().w(viewport.width).h(viewport.height).child(card)),
    )
    .into_any_element()
}
