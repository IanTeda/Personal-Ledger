//! The Transactions page's Views (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a): a
//! full-width view -- the header block, the column header, then the virtualised table -- with no
//! context rail beside it (the mockup shows none, the same as Accounts and Settings).
//!
//! `page.rs` wraps the page Presenter in an Entity, which the Shell hosts as the Transactions
//! destination; this module draws its view model: the `header` (title, count line, add button,
//! chip row, search), the `table` (column header and virtualised rows) and the `footer`, stacked as
//! a fixed header, a fixed column header, the one flexing list and a fixed footer, with the filter
//! popover drawn over the window while it is open. The Import step shows in its place while it is
//! open (`crate::transaction_import`). Every click is a call into the page Entity, so a click and
//! a key reach the Presenter alike.

mod filter_popover;
mod footer;
mod header;
pub(crate) mod hints;
pub mod page;
pub mod snapshot;
pub(crate) mod table;

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Pixels, UniformListScrollHandle, Window, div, point, prelude::*, px,
};

use lib_transactions::presenter::{
    ChipField, FilterEvent, FilterViewModel, FooterView, RowView, TransactionsPageEvent,
};

use crate::{settings::display::StatusGlyphs, view::format};
use filter_popover::{OnFieldClick, OnOptionClick, OnStatusClick, PopoverProps};
use header::{HeaderProps, OnChipClick, OnPlainClick};
use page::TransactionsPage;
use table::OnRowClick;

struct TransactionsPageProps {
    /// The filter popover is open: the page dims to 55% (no blur in gpui 0.2, so opacity only).
    dimmed: bool,
    header: HeaderProps,
    /// Every visible row, formatted per the Display preferences.
    rows: Rc<Vec<RowView>>,
    /// A position in `rows`, already clamped.
    selected: usize,
    /// The height of every row, from the Display density preference.
    row_height: Pixels,
    glyphs: StatusGlyphs,
    scroll: UniformListScrollHandle,
    on_row_click: OnRowClick,
    footer: FooterView,
}

/// The page: the fixed header block, the fixed column header, the one flexing list, the fixed footer.
fn render(props: TransactionsPageProps, cx: &App) -> AnyElement {
    div()
        .id("transactions")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .when(props.dimmed, |this| this.opacity(0.55))
        .child(header::render(props.header, cx))
        .child(table::column_header(cx))
        .child(table::rows(
            props.rows,
            props.selected,
            props.row_height,
            props.glyphs,
            props.scroll,
            props.on_row_click,
            cx,
        ))
        .child(footer::render(&props.footer, cx))
        .into_any_element()
}

impl Render for TransactionsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let event = |event: TransactionsPageEvent| {
            let entity = entity.clone();
            let on_click: OnPlainClick = Rc::new(move |_window, cx| {
                let event = event.clone();
                entity.update(cx, |page, cx| page.click(event, cx));
            });
            on_click
        };
        let chip = |make: fn(ChipField) -> Option<TransactionsPageEvent>| {
            let entity = entity.clone();
            let on_click: OnChipClick = Rc::new(move |field, _window, cx| {
                entity.update(cx, |page, cx| match make(field) {
                    Some(event) => page.click(event, cx),
                    None => page.click_chip(field, cx),
                });
            });
            on_click
        };
        let on_search_click: OnPlainClick = {
            let entity = entity.clone();
            Rc::new(move |_window, cx| entity.update(cx, TransactionsPage::click_search))
        };
        let on_row_click: OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |page, cx| {
                    page.click(TransactionsPageEvent::Select(index), cx);
                });
            })
        };

        let view = self.view_model();
        let popover = view
            .filter
            .clone()
            .map(|filter| self.popover(filter, window, &entity, cx));
        let cx: &App = cx;
        let page = render(
            TransactionsPageProps {
                dimmed: view.filter.is_some(),
                header: HeaderProps {
                    count_line: view.count_line,
                    chips: view.chips,
                    show_clear: view.filters_changed,
                    search: view.search,
                    searching: view.searching,
                    on_add_click: event(TransactionsPageEvent::New),
                    on_chip_click: chip(|_| None),
                    on_chip_clear: chip(|field| Some(TransactionsPageEvent::ClearChip(field))),
                    on_clear_all: event(TransactionsPageEvent::ClearFilters),
                    on_search_click,
                    chip_bounds: self.chip_bounds.clone(),
                },
                selected: view.selected,
                rows: Rc::new(view.rows),
                row_height: px(format::row_height_px(self.density)),
                glyphs: self.glyphs,
                scroll: self.scroll.clone(),
                on_row_click,
                footer: view.footer,
            },
            cx,
        );
        div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .child(page)
            .children(popover)
            .into_any_element()
    }
}

impl TransactionsPage {
    /// The filter popover, drawn over the whole window (its cancelling layer covers the rails too)
    /// and anchored just below the chip that opened it, left-aligned to it and kept inside the
    /// window; before any chip has been painted, at a fixed spot.
    fn popover(
        &self,
        view: FilterViewModel,
        window: &Window,
        entity: &gpui::Entity<Self>,
        cx: &App,
    ) -> AnyElement {
        let filter = |event: FilterEvent| {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut App| {
                let event = TransactionsPageEvent::Filter(event.clone());
                entity.update(cx, |page, cx| page.click(event, cx));
            }) as crate::dialog::OnClick
        };
        let on_field_click: OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |view, cx| view.click_filter_field(field, cx));
            })
        };
        let on_option_click: OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |field, index, _window, cx| {
                entity.update(cx, |view, cx| view.click_filter_option(field, index, cx));
            })
        };
        let on_status_click: OnStatusClick = {
            let entity = entity.clone();
            Rc::new(move |status, _window, cx| {
                entity.update(cx, |view, cx| view.click_filter_status(status, cx));
            })
        };
        let viewport = window.viewport_size();
        let anchor = self.chip_bounds.borrow().get(&self.filter_anchor).copied();
        let (left, top) = match anchor {
            Some(bounds) => {
                let left = bounds.origin.x.min(viewport.width - px(416.0)).max(px(8.0));
                (left, bounds.origin.y + bounds.size.height + px(6.0))
            }
            None => (px(300.0), px(200.0)),
        };
        let card = filter_popover::render(
            PopoverProps {
                view,
                list: self.filter_list.as_ref(),
                left,
                top,
                on_field_click,
                on_option_click,
                on_status_click,
                on_reset: filter(FilterEvent::Reset),
                on_apply: filter(FilterEvent::Apply),
                on_cancel: filter(FilterEvent::Cancel),
            },
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
}
