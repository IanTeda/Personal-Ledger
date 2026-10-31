//! The Import step's View: `lib-transaction-import`'s Presenter in a gpui Entity, which the Shell
//! shows in place of the Transactions list while the step is open (ADR-0037). It looks up each key
//! the Shell passes on, as chord text, in the step's keymap (the open select's while one is open),
//! turns clicks into events, and draws the view model (`super`). Each key and click is one input
//! (`crate::input::send`), which commits what it wrote.
//!
//! What stays here is gpui's: the rows' scroll and the open select's window of options. The
//! Presenter is built as the step opens, reading the Payees and Categories its selects offer then,
//! and dropped as it closes.

use std::ops::Range;

use gpui::{App, Context, Entity, ScrollHandle};
use lib_core::Clock;
use lib_transaction_import::keymap::{import_keymap, import_select_keymap};
use lib_transaction_import::presenter::{
    ImportEvent, ImportPresenter, ImportViewModel, RowSelect, legend,
};
use lib_viper::keymap::Keymap;

use crate::form::select::MAX_VISIBLE_ROWS;
use crate::ledger_store::LedgerStore;
use crate::shell::view::statusline::PageStatus;
use crate::transaction_import::ImportStores;
use crate::transaction_import::router::PageRouter;

/// The Import step's Entity.
pub struct ImportPage {
    stores: ImportStores,
    ledger: Entity<LedgerStore>,
    clock: Clock,
    /// The step's Presenter, `Some` while it shows in place of the Transactions list (`:import`).
    presenter: Option<ImportPresenter>,
    keymap: Keymap<ImportEvent>,
    /// The keys while a row's select is open.
    select_keymap: Keymap<ImportEvent>,
    /// The step's own scroll, reset as it opens and closes.
    pub(super) scroll: ScrollHandle,
    /// The first option the open select draws, so its window follows the highlight.
    select_window: usize,
}

impl ImportPage {
    /// The step, closed, over `stores`, under the default keys until `Root` pushes
    /// `[keybindings]`. `clock` dates the statement as the step opens.
    pub fn new(stores: ImportStores, ledger: Entity<LedgerStore>, clock: Clock) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self {
            stores,
            ledger,
            clock,
            presenter: None,
            keymap: import_keymap(&config),
            select_keymap: import_select_keymap(&config),
            scroll: ScrollHandle::new(),
            select_window: 0,
        }
    }

    /// Follows a change to `[keybindings]`.
    pub fn set_keymaps(&mut self, config: &lib_config::KeyBindingConfig) {
        self.keymap = import_keymap(config);
        self.select_keymap = import_select_keymap(config);
    }

    /// Whether the step shows in place of the Transactions list.
    pub fn is_open(&self) -> bool {
        self.presenter.is_some()
    }

    /// The step as drawn; `None` while it is closed.
    pub fn view_model(&self) -> Option<ImportViewModel> {
        self.presenter.as_ref().map(ImportPresenter::view_model)
    }

    /// `:import`: opens the step on the seeded statement, matched against the stored Payees. Not
    /// an input of its own: it writes nothing, and the palette Command that opened it is the input.
    pub fn open(&mut self, cx: &mut Context<'_, Self>) {
        self.presenter = Some(ImportPresenter::new(self.stores.ports(&self.clock)));
        self.reset_scroll();
        cx.notify();
    }

    fn close(&mut self) {
        self.presenter = None;
        self.reset_scroll();
    }

    fn reset_scroll(&mut self) {
        self.scroll.set_offset(gpui::Point::default());
        self.select_window = 0;
    }

    /// Transactions has just become the destination: a step left open when it was left doesn't
    /// come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.close();
        cx.notify();
    }

    /// A key the step's keymap binds, by `chord`: the open select's while one is open, which
    /// swallows every other key as an open list does in the Dialogs, else the rows'. `esc` is the
    /// Shell's, and comes back as [`Self::escape`].
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(presenter) = self.presenter.as_ref() else {
            return false;
        };
        let select_open = presenter.view_model().open_select.is_some();
        let keymap = if select_open {
            &self.select_keymap
        } else {
            &self.keymap
        };
        match keymap.event(chord) {
            Some(event) => self.send(event, cx),
            None => return select_open,
        }
        true
    }

    /// `esc`: closes an open select, else leaves the step for the Transactions list.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.presenter.is_none() {
            return false;
        }
        self.send(ImportEvent::Cancel, cx);
        true
    }

    /// A click on a row selects it, closing another row's open select.
    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.send(ImportEvent::SelectRow(index), cx);
    }

    /// A click on a row's select: opens it, or closes it when it is already the open one.
    pub(super) fn click_select(
        &mut self,
        index: usize,
        which: RowSelect,
        cx: &mut Context<'_, Self>,
    ) {
        let open = self
            .view_model()
            .and_then(|view| view.open_select)
            .is_some_and(|open| open.row == index && open.which == which);
        let event = if open {
            ImportEvent::Cancel
        } else {
            ImportEvent::OpenRow(index, which)
        };
        self.send(event, cx);
    }

    pub(super) fn click_option(&mut self, option: usize, cx: &mut Context<'_, Self>) {
        self.send(ImportEvent::Choose(option), cx);
    }

    pub(super) fn click_remember(&mut self, cx: &mut Context<'_, Self>) {
        self.send(ImportEvent::ToggleRemember, cx);
    }

    /// **back** leaves the step, closing an open select on the way.
    pub(super) fn click_back(&mut self, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| {
            if self
                .view_model()
                .is_some_and(|view| view.open_select.is_some())
            {
                self.handle(ImportEvent::Cancel, cx);
            }
            self.handle(ImportEvent::Cancel, cx);
        });
    }

    /// **continue**: commits the step, as `enter` does.
    pub(super) fn click_continue(&mut self, cx: &mut Context<'_, Self>) {
        self.send(ImportEvent::Continue, cx);
    }

    /// A key or click as one input, committed once it is handled.
    fn send(&mut self, event: ImportEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter, then follows what it did: the selected row into view, the
    /// open select's window onto its highlight, and a close. A commit refreshes the Ledger's read
    /// copies and lands on the Transactions list's first row, inside the same input.
    fn handle(&mut self, event: ImportEvent, cx: &mut Context<'_, Self>) {
        let Some(presenter) = self.presenter.as_mut() else {
            return;
        };
        let was_open = presenter
            .view_model()
            .open_select
            .map(|open| (open.row, open.which));
        let mut router = PageRouter::default();
        presenter.handle(event, &mut router);
        match router.closed {
            Some(committed) => {
                self.close();
                if committed.is_some() {
                    super::super::refresh(&self.ledger, cx);
                    crate::transactions::router::first_row(cx);
                }
            }
            None => {
                let view = presenter.view_model();
                if let ImportEvent::Move(_) = event {
                    self.scroll.scroll_to_item(view.selected);
                }
                let open = view.open_select.as_ref().map(|open| (open.row, open.which));
                if open != was_open {
                    self.select_window = 0;
                }
                if let Some(open) = &view.open_select {
                    self.select_window = follow(
                        self.select_window,
                        open.highlight,
                        open.select.options.len(),
                    );
                }
            }
        }
        cx.notify();
    }

    /// The open select's options to draw: at most [`MAX_VISIBLE_ROWS`], from where its window has
    /// scrolled to.
    pub(super) fn select_range(&self, len: usize) -> Range<usize> {
        let start = self.select_window.min(len.saturating_sub(MAX_VISIBLE_ROWS));
        start..(start + MAX_VISIBLE_ROWS).min(len)
    }

    /// The status line's legend, each chord read from the keymap in use so an override shows, and
    /// whether rows still need review.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let view = self.view_model()?;
        let keymap = if view.open_select.is_some() {
            &self.select_keymap
        } else {
            &self.keymap
        };
        Some(PageStatus {
            hints: legend(&view.hints, keymap),
            right: view.status,
        })
    }
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
