//! The Transactions page's View: `lib-transactions`' page Presenter's [`PageView`], which the
//! generic `ModulePage` hosts as the Transactions destination (ADR-0056,
//! `docs/ux/desktop-mockups/03-transactions/`). The keymaps (the page's, the Search's and the filter
//! popover's), the Search and the popover as the page's modes, the popover's open select and every
//! event are the Presenter's; it draws the view model (`mod.rs`, `header.rs`, `table.rs`,
//! `footer.rs`, `filter_popover.rs`).
//!
//! What stays here is gpui's: the table's scroll, which [`PageView::before`] measures the half page
//! from and [`PageView::after`] follows the selection with, where each chip was painted and which
//! one the popover anchors under, and the open select's window of options.
//!
//! The rows name Accounts, Categories, Payees, Tags and Units, read through the stores, so the page
//! repaints on every `LedgerChanged` naming any of them.

use gpui::{AnyElement, Context, ScrollStrategy, UniformListScrollHandle, Window};

use lib_core::Clock;
use lib_database::Domain;
use lib_list_cursor::Step;
use lib_transactions::SharedPorts;
use lib_transactions::presenter::{
    ChipField, TransactionsPageEvent, TransactionsPagePresenter, TransactionsPageViewModel,
};
use lib_viper::display::RowDensity;

use super::header::ChipBounds;
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::{event::ViewEvent, format, help::HelpGroup};

/// The Transactions page: its Presenter in the generic page.
pub type TransactionsPage = ModulePage<TransactionsPagePresenter>;

/// The table's scroll, the chips' painted bounds and the popover's anchor and select window.
#[derive(Default)]
pub struct TransactionsWidgets {
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    pub(super) scroll: UniformListScrollHandle,
    /// The chip that opened the popover, which it anchors under; Account until one has.
    pub(super) filter_anchor: Option<ChipField>,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    pub(super) chip_bounds: ChipBounds,
    /// The first drawn option of the popover's open select.
    pub(super) select_window: usize,
}

/// How a handle moves the table's scroll.
#[derive(Default)]
enum Scroll {
    /// Onto a row that changed, or back to the top on the first row.
    #[default]
    Follow,
    /// A row step, by the strategy its direction lands the row with.
    Move(ScrollStrategy),
    /// A revealed row, centred.
    Reveal,
    /// The page entered again, its scroll starting afresh.
    Entered,
}

/// What a handle is compared against: the selection and density before it, how it scrolls, and
/// the chip a popover opens under.
#[derive(Default)]
pub struct TransactionsBefore {
    selected: usize,
    density: RowDensity,
    scroll: Scroll,
    anchor: Option<ChipField>,
}

impl PageView for TransactionsPagePresenter {
    type Setup = ();
    type Widgets = TransactionsWidgets;
    type Before = TransactionsBefore;

    const REPAINTS: &'static [Domain] = &[
        Domain::Accounts,
        Domain::Categories,
        Domain::Payees,
        Domain::Tags,
        Domain::Transactions,
        Domain::Units,
    ];

    /// The table draws every column, scrolling sideways in a narrower View.
    fn min_width() -> Option<gpui::Pixels> {
        Some(crate::transactions::view::table::min_width())
    }

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        _cx: &mut Context<'_, TransactionsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut Queue)
    }

    /// A row step is measured against the half page the table shows now.
    fn before(
        &mut self,
        event: &TransactionsPageEvent,
        widgets: &TransactionsWidgets,
    ) -> TransactionsBefore {
        let scroll = match event {
            TransactionsPageEvent::Move(step) => {
                self.set_half_page(half_page(&widgets.scroll, self.density()));
                Scroll::Move(match step {
                    Step::Next | Step::Last => ScrollStrategy::Bottom,
                    Step::Prev | Step::First => ScrollStrategy::Top,
                    Step::HalfPageDown | Step::HalfPageUp => ScrollStrategy::Center,
                })
            }
            TransactionsPageEvent::RevealTransaction(_) => Scroll::Reveal,
            TransactionsPageEvent::Entered => Scroll::Entered,
            _ => Scroll::Follow,
        };
        // A key or palette Command opens the popover at Account, as `f` does; a chip under itself.
        let anchor = match event {
            TransactionsPageEvent::OpenFilter(chip) => Some(chip.unwrap_or(ChipField::Account)),
            _ => None,
        };
        TransactionsBefore {
            selected: self.selected(),
            density: self.density(),
            scroll,
            anchor,
        }
    }

    /// A non-strict scroll applies its strategy only when the row was off-screen, so a step down
    /// lands the row at the bottom and a step up at the top; a reveal centres its row.
    fn after(
        &self,
        before: TransactionsBefore,
        _setup: &Self::Setup,
        widgets: &mut TransactionsWidgets,
    ) -> Option<ViewEvent> {
        if let Some(anchor) = before.anchor {
            widgets.filter_anchor = Some(anchor);
        }
        let now = self.selected();
        if self.density() != before.density {
            // The table's scroll offset is in pixels, so a new row height would leave it pointing
            // at a different row: re-anchor on the selected one for its next paint.
            widgets
                .scroll
                .scroll_to_item_strict(now, ScrollStrategy::Center);
        }
        match before.scroll {
            Scroll::Reveal => widgets
                .scroll
                .scroll_to_item_strict(now, ScrollStrategy::Center),
            Scroll::Move(strategy) => widgets.scroll.scroll_to_item(now, strategy),
            // The scroll started afresh as the page was entered, so the row it kept is found again.
            Scroll::Entered if now != 0 => widgets
                .scroll
                .scroll_to_item_strict(now, ScrollStrategy::Center),
            _ if now != before.selected || now == 0 => {
                widgets.scroll.scroll_to_item(now, ScrollStrategy::Top);
            }
            _ => {}
        }
        None
    }

    /// A Search or popover left open when the page was left doesn't come back with it.
    fn entered() -> Option<TransactionsPageEvent> {
        Some(TransactionsPageEvent::Entered)
    }

    /// The `?` cheat-sheet's Transactions group: the keys the status line shows.
    fn help(view: &TransactionsPageViewModel) -> Option<HelpGroup> {
        Some(HelpGroup {
            heading: crate::msg::desktop_help_transactions_label(),
            keys: view
                .hints
                .iter()
                .map(|hint| (hint.action.clone(), hint.chord.clone()))
                .collect(),
        })
    }

    fn render(
        view: &TransactionsPageViewModel,
        page: PageDraw<'_, Self>,
        window: &mut Window,
        cx: &mut Context<'_, TransactionsPage>,
    ) -> AnyElement {
        super::render(view.clone(), page.widgets, page.clicks, window, cx)
    }

    /// The status line's legend (the filter popover's or the page's), and the status marks.
    fn status(view: &TransactionsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.legend.clone().into(),
        }
    }

    /// `Esc`: the popover's open list first, then the popover itself (its draft discarded), then
    /// the Search (its text cleared).
    fn escape(view: &TransactionsPageViewModel) -> Option<TransactionsPageEvent> {
        if view.filter.is_some() {
            Some(TransactionsPageEvent::Filter(
                lib_transactions::presenter::FilterEvent::Back,
            ))
        } else if view.searching {
            Some(TransactionsPageEvent::ClearSearch)
        } else {
            None
        }
    }
}

/// The Transactions module's entry point: the page on `ports`, filtered to this year by `clock`'s
/// today, under the default keys until `Root` pushes `[keybindings]`.
pub fn transactions_page(
    ports: SharedPorts,
    announcer: &gpui::Entity<Announcer>,
    clock: Clock,
    cx: &mut gpui::App,
) -> gpui::Entity<TransactionsPage> {
    let presenter =
        TransactionsPagePresenter::new(ports, clock, &lib_config::KeyBindingConfig::default());
    gpui::AppContext::new(cx, |cx| ModulePage::new(presenter, (), announcer, cx))
}

/// How many rows a half-page move steps, from the list's measured viewport height.
fn half_page(scroll: &UniformListScrollHandle, density: RowDensity) -> usize {
    let viewport = f32::from(scroll.0.borrow().base_handle.bounds().size.height);
    half_page_rows(viewport, format::row_height_px(density))
}

/// How many rows a half-page move steps, from the list's measured viewport height. Before the first
/// paint the height is zero, so it falls back to a small fixed step rather than moving nowhere.
fn half_page_rows(viewport_px: f32, row_px: f32) -> usize {
    if viewport_px <= 0.0 || row_px <= 0.0 {
        return lib_list_cursor::HALF_PAGE;
    }
    (((viewport_px / row_px).floor() as usize) / 2).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_half_page_is_half_the_rows_that_fit_and_never_zero() {
        assert_eq!(half_page_rows(680.0, 34.0), 10);
        assert_eq!(half_page_rows(60.0, 34.0), 1);
        assert_eq!(half_page_rows(0.0, 34.0), 5, "before the first paint");
        assert_eq!(half_page_rows(680.0, 0.0), 5);
    }
}
