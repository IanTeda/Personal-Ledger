//! The Import step's View: `lib-transaction-import`'s step Presenter's [`PageView`], which the
//! generic `ModulePage` hosts in place of the Transactions list while the step is open (ADR-0056).
//! The step's state, open or closed, its keymaps (the open select's while one is open) and every
//! event are the Presenter's; it draws the view model (`super`).
//!
//! What stays here is gpui's: the rows' scroll and the open select's window of options, which
//! follow each handle in [`PageView::after`] and start afresh as the step opens or closes.

use std::ops::Range;

use gpui::{AnyElement, Context, ScrollHandle, Window, prelude::*};
use lib_core::Clock;
use lib_database::Domain;
use lib_transaction_import::presenter::RowSelect;
use lib_transaction_import::presenter::{ImportEvent, ImportStepPresenter, ImportStepViewModel};
use lib_viper::presenter::Presenter as _;

use crate::form::select::MAX_VISIBLE_ROWS;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView};
use crate::shell::view::statusline::PageStatus;
use crate::transaction_import::ImportStores;
use crate::transaction_import::router::PageRouter;
use crate::view::event::ViewEvent;

/// The Import step: its Presenter in the generic page.
pub type ImportPage = ModulePage<ImportStepPresenter>;

/// The step's scroll and the open select's first drawn option.
#[derive(Default)]
pub struct ImportWidgets {
    scroll: ScrollHandle,
    select_window: usize,
}

/// What a handle is compared against: whether the step was open, which select was, and whether
/// the event moves the selection.
#[derive(Default)]
pub struct ImportBefore {
    open: bool,
    select: Option<(usize, RowSelect)>,
    moves: bool,
}

impl PageView for ImportStepPresenter {
    type Setup = ();
    type Widgets = ImportWidgets;
    type Before = ImportBefore;

    /// The step reads the Payees and Categories as it opens, and nothing changes them under it.
    const REPAINTS: &'static [Domain] = &[];

    /// The step keeps the Transactions table's width, which it shows in place of.
    fn min_width() -> Option<gpui::Pixels> {
        Some(crate::transactions::view::table::min_width())
    }

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        cx: &mut Context<'_, ImportPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { cx })
    }

    fn before(&mut self, event: &ImportEvent, _widgets: &ImportWidgets) -> ImportBefore {
        ImportBefore {
            open: self.is_open(),
            select: open_select(&self.view_model()),
            moves: matches!(event, ImportEvent::Move(_)),
        }
    }

    /// A step opened or closed starts at the top; an open one keeps its selected row in view and
    /// the open select's window on its highlight.
    fn after(&self, before: ImportBefore, widgets: &mut ImportWidgets) -> Option<ViewEvent> {
        if self.is_open() != before.open {
            *widgets = ImportWidgets::default();
            return None;
        }
        let view = self.view_model().step?;
        if before.moves {
            widgets.scroll.scroll_to_item(view.selected);
        }
        let open = view.open_select.as_ref().map(|open| (open.row, open.which));
        if open != before.select {
            widgets.select_window = 0;
        }
        if let Some(open) = &view.open_select {
            widgets.select_window = follow(
                widgets.select_window,
                open.highlight,
                open.select.options.len(),
            );
        }
        None
    }

    /// A step left open when Transactions was left doesn't come back with it.
    fn entered() -> Option<ImportEvent> {
        Some(ImportEvent::Leave)
    }

    fn render(
        view: &ImportStepViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, ImportPage>,
    ) -> AnyElement {
        let Some(step) = &view.step else {
            return gpui::Empty.into_any_element();
        };
        let select_range = step
            .open_select
            .as_ref()
            .map(|open| select_range(page.widgets.select_window, open.select.options.len()));
        super::render(
            step,
            view.continue_chord.clone(),
            select_range,
            &page.widgets.scroll,
            page.clicks,
            cx,
        )
    }

    /// The legend for the keymap in use, and whether rows still need review.
    fn status(view: &ImportStepViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view
                .step
                .as_ref()
                .map(|step| step.status.clone())
                .unwrap_or_default(),
        }
    }

    /// `esc`: closes an open select, else leaves the step for the Transactions list.
    fn escape(view: &ImportStepViewModel) -> Option<ImportEvent> {
        view.step.as_ref().map(|_| ImportEvent::Cancel)
    }
}

/// The open select's row and which select it is.
fn open_select(view: &ImportStepViewModel) -> Option<(usize, RowSelect)> {
    let open = view.step.as_ref()?.open_select.as_ref()?;
    Some((open.row, open.which))
}

/// The Transaction Import module's entry point: the step, closed, over `stores`, under the default
/// keys until `Root` pushes `[keybindings]`. `clock` dates the statement as the step opens.
pub fn import_page(
    stores: ImportStores,
    announcer: &gpui::Entity<crate::announcer::Announcer>,
    clock: Clock,
    cx: &mut gpui::App,
) -> gpui::Entity<ImportPage> {
    let presenter = ImportStepPresenter::new(
        move || stores.ports(&clock),
        &lib_config::KeyBindingConfig::default(),
    );
    cx.new(|cx| ModulePage::new(presenter, (), announcer, cx))
}

/// The open select's options to draw: at most [`MAX_VISIBLE_ROWS`] of `len`, from `start`.
fn select_range(start: usize, len: usize) -> Range<usize> {
    let start = start.min(len.saturating_sub(MAX_VISIBLE_ROWS));
    start..(start + MAX_VISIBLE_ROWS).min(len)
}

/// The first option of a window of [`MAX_VISIBLE_ROWS`] over `len` options, moved from `start`
/// only as far as it takes to show `highlight`.
fn follow(start: usize, highlight: usize, len: usize) -> usize {
    let start = if highlight < start {
        highlight
    } else if highlight >= start + MAX_VISIBLE_ROWS {
        highlight + 1 - MAX_VISIBLE_ROWS
    } else {
        start
    };
    start.min(len.saturating_sub(MAX_VISIBLE_ROWS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_select_window_moves_only_as_far_as_the_highlight() {
        assert_eq!(follow(0, 3, 20), 0, "inside the window");
        assert_eq!(follow(0, MAX_VISIBLE_ROWS, 20), 1, "one past the end");
        assert_eq!(follow(5, 2, 20), 2, "above the window");
        assert_eq!(follow(0, 3, 4), 0, "a short list never scrolls");
    }
}
