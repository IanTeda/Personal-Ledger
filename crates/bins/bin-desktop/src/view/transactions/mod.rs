//! The **4a** Transactions page (`docs/ux/desktop-mockups/03-transactions/README.md`): a full-width view --
//! the header block, the column header, then the virtualised table -- with no context rail beside
//! it (the mockup shows none, the same as Accounts and Settings).
//!
//! This module owns the page's frame: the `header` (title, count line, add button, chip row,
//! search), the `table` (column header and virtualised rows) and the `footer`, stacked as a fixed
//! header, a fixed column header, the one flexing list and a fixed footer, with the filter popover
//! drawn over the window while it is open. The Import View shows in its place while it is open
//! (`view::import`).
//!
//! [`state::TransactionsView`] renders the page itself (ADR-0037): every action is a call back into
//! the Transactions Entity, so keyboard and mouse reach the same methods.

mod filter_popover;
mod footer;
mod header;
pub(crate) mod hints;
pub mod snapshot;
pub mod state;
pub(crate) mod table;

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Pixels, UniformListScrollHandle, Window, div, point, prelude::*, px,
};

use lib_transactions::presenter::{ChipField, FilterPresenter, FilterViewModel};

use crate::{
    transactions::{self, rows::RowView, summary::Footer},
    view::format,
};
use filter_popover::{OnFieldClick, OnOptionClick, OnStatusClick, PopoverProps};
use header::{HeaderProps, OnChipClick, OnPlainClick};
use state::TransactionsView;
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
    scroll: UniformListScrollHandle,
    on_row_click: OnRowClick,
    footer: Footer,
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
            props.scroll,
            props.on_row_click,
            cx,
        ))
        .child(footer::render(&props.footer, cx))
        .into_any_element()
}

impl Render for TransactionsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let plain = |handler: fn(&mut TransactionsView, &mut Context<'_, TransactionsView>)| {
            let entity = entity.clone();
            let on_click: OnPlainClick = Rc::new(move |_window, cx| entity.update(cx, handler));
            on_click
        };
        let indexed =
            |handler: fn(&mut TransactionsView, usize, &mut Context<'_, TransactionsView>)| {
                let entity = entity.clone();
                let on_click: OnRowClick = Rc::new(move |index, _window, cx| {
                    entity.update(cx, |view, cx| handler(view, index, cx));
                });
                on_click
            };
        let chip =
            |handler: fn(&mut TransactionsView, ChipField, &mut Context<'_, TransactionsView>)| {
                let entity = entity.clone();
                let on_click: OnChipClick = Rc::new(move |field, _window, cx| {
                    entity.update(cx, |view, cx| handler(view, field, cx));
                });
                on_click
            };

        let popover = self
            .filter_view()
            .map(|view| self.popover(view, window, &entity, cx));
        let cx: &App = cx;
        let state = &self.state;
        let ledger = self.ledger(cx);
        let visible = transactions::query::query(
            &ledger,
            self.transactions(cx),
            &state.filters,
            &state.search,
        );
        let rows = transactions::rows::build_rows(&visible, &ledger, &self.prefs());
        let footer = transactions::summary::footer(&visible, &state.filters, &ledger);
        let chips = FilterPresenter::chips(&ledger, &state.filters, self.today, self.date_style);
        let page = render(
            TransactionsPageProps {
                dimmed: state.filter.is_some(),
                header: HeaderProps {
                    count_line: transactions::summary::count_line(self.transactions(cx)),
                    chips,
                    show_clear: !state.filters.is_default(self.today),
                    search: state.search.clone(),
                    searching: self.searching,
                    on_add_click: plain(TransactionsView::click_add),
                    on_chip_click: chip(TransactionsView::click_chip),
                    on_chip_clear: chip(TransactionsView::clear_chip),
                    on_clear_all: plain(TransactionsView::clear_all),
                    on_search_click: plain(TransactionsView::click_search),
                    chip_bounds: state.chip_bounds.clone(),
                },
                selected: transactions::rows::clamp_selection(state.selected, rows.len()),
                rows: Rc::new(rows),
                row_height: px(format::row_height_px(self.density)),
                scroll: state.scroll.clone(),
                on_row_click: indexed(TransactionsView::click_row),
                footer,
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

impl TransactionsView {
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
        let plain = |handler: fn(&mut TransactionsView, &mut Context<'_, TransactionsView>)| {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut App| entity.update(cx, handler))
                as crate::dialog::OnClick
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
        let anchor = self
            .state
            .chip_bounds
            .borrow()
            .get(&self.state.filter_anchor)
            .copied();
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
                list: self.state.filter_list.as_ref(),
                left,
                top,
                on_field_click,
                on_option_click,
                on_status_click,
                on_reset: plain(TransactionsView::reset_filter_form),
                on_apply: plain(TransactionsView::apply_filter_form),
                on_cancel: plain(TransactionsView::close_filter_popover),
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
