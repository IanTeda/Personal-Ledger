//! The Transactions destination's Import View (#924): a legacy Entity (ADR-0037) the root Router
//! shows in place of the Transactions list while it is open, the Noun staying Transactions. It
//! reads the Payees and Categories its selects draw on from the `Ledger` read copy and commits
//! through `LedgerStore::apply`, until `lib-transaction-import` makes Import a module (#810). The
//! pure rules live in `crate::import` and the drawing in `super`.

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription};
use lib_ledger::{CommitImport, Domain};
use lib_list_cursor::Step;
use lib_payees::Payee;
use lib_toast::{Notice, Notify as _, ToastKind};

use super::import_hints;
use crate::{
    import::{self, ImportState, RowSelect},
    ledger_store::{LedgerStore, notify_on},
    notices::Queue,
    shell::view::statusline::{PageStatus, owned_hints},
};

/// An Import committed: the root Router puts the Transactions list back on its first row, where
/// the person lands.
pub struct ImportCommitted;

impl EventEmitter<ImportCommitted> for ImportView {}

/// The Import View's Entity.
pub struct ImportView {
    ledger: Entity<LedgerStore>,
    today: NaiveDate,
    /// The stubbed 6e "match payees" step, `Some` while it shows in place of the Transactions list
    /// (`:import`). Dropped when Transactions becomes the destination again.
    pub(super) state: Option<ImportState>,
    /// The step's own scroll, reset as it opens and closes.
    pub(super) scroll: ScrollHandle,
    /// Repaint the step when the Payees and Categories its selects offer change.
    _payees_changed: Subscription,
    _categories_changed: Subscription,
}

impl ImportView {
    pub fn new(
        ledger: Entity<LedgerStore>,
        clock: &lib_core::Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let _payees_changed = notify_on(&ledger, Domain::Payees, cx);
        let _categories_changed = notify_on(&ledger, Domain::Categories, cx);
        Self {
            ledger,
            today: clock.today(),
            state: None,
            scroll: ScrollHandle::new(),
            _payees_changed,
            _categories_changed,
        }
    }

    /// Whether the step shows in place of the Transactions list.
    pub fn is_open(&self) -> bool {
        self.state.is_some()
    }

    pub(super) fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.ledger.read(cx).ledger().payees()
    }

    /// The Category select's options: "choose category…", then every leaf.
    pub(super) fn categories(&self, cx: &App) -> import::CategoryOptions {
        import::CategoryOptions::new(
            self.ledger.read(cx).payee_categories(),
            crate::msg::desktop_import_choose_category(),
        )
    }

    /// `:import`: opens the step on the seeded statement.
    pub fn open(&mut self, cx: &mut Context<'_, Self>) {
        self.state = Some(ImportState::new(self.payees(cx), self.today));
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    fn close(&mut self) {
        self.state = None;
        self.scroll.set_offset(gpui::Point::default());
    }

    /// Transactions has just become the destination: a step left open when it was left doesn't
    /// come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.close();
        cx.notify();
    }

    /// The step's keys: `j`/`k` move the row, `p` and `c` open its Payee and Category selects, `n`
    /// creates a new Payee from its cleaned name, `r` toggles "remember" and `enter` continues.
    /// While a select is open it owns `j`/`k`/arrows and `enter`/`space`. Any other key falls
    /// through to the Shell's fallback; `esc` comes back as [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let key = keystroke.key.as_str();
        if key == "escape" {
            return false;
        }
        let categories = self.categories(cx);
        let payees = self.payees(cx).to_vec();
        let Some(state) = self.state.as_mut() else {
            return false;
        };
        let len = state.rows.len();
        let selected = state.selected.min(len.saturating_sub(1));
        let Some(choices) = state
            .rows
            .get(selected)
            .map(|row| super::payee_choices(&payees, &row.raw))
        else {
            return false;
        };

        if state.open_select.is_some() {
            let select_key = match key {
                "j" | "down" => import::SelectKey::Down,
                "k" | "up" => import::SelectKey::Up,
                "enter" | "space" => import::SelectKey::Commit,
                // An open list swallows everything else, like the dialogs' selects.
                _ => return true,
            };
            state.handle_select_key(select_key, &payees, &choices, &categories);
            cx.notify();
            return true;
        }

        match key {
            "enter" => {
                self.continue_import(cx);
                return true;
            }
            "j" | "down" => state.selected = lib_list_cursor::step(selected, len, Step::Next),
            "k" | "up" => state.selected = lib_list_cursor::step(selected, len, Step::Prev),
            "p" => state.open(selected, RowSelect::Payee, &choices, &categories),
            "c" => state.open(selected, RowSelect::Category, &choices, &categories),
            "n" => state.create_new_payee(&payees),
            "r" => state.remember = !state.remember,
            _ => return false,
        }
        let selected = state.selected;
        self.scroll.scroll_to_item(selected);
        cx.notify();
        true
    }

    /// `esc`: closes an open select, else goes back to the Transactions list.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let categories = self.categories(cx);
        let payees = self.payees(cx).to_vec();
        let Some(state) = self.state.as_mut() else {
            return false;
        };
        if state.open_select.is_some() {
            let choices = state
                .rows
                .get(state.selected.min(state.rows.len().saturating_sub(1)))
                .map(|row| super::payee_choices(&payees, &row.raw));
            if let Some(choices) = choices {
                state.handle_select_key(import::SelectKey::Cancel, &payees, &choices, &categories);
            }
        } else {
            self.close();
        }
        cx.notify();
        true
    }

    /// **continue** / `enter`: commits the import to the Ledger and lands on the Transactions list
    /// with a Toast, as one input (`crate::input::send`): the booking commits once, and the Toast
    /// is queued as a Notice that input drains. Nothing writes the Account: its Dialogs count the
    /// Transactions. Does nothing while a row needs review (the button is disabled then).
    pub(super) fn continue_import(&mut self, cx: &mut Context<'_, Self>) {
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let bookings = match import::bookings(&state.rows, state.remember) {
            Ok(bookings) => bookings,
            Err(import::ImportError::NeedsReview(_)) => return,
        };
        crate::input::send(cx, |cx| {
            let committed = self.ledger.update(cx, |store, cx| {
                store.apply(
                    cx,
                    CommitImport {
                        account_id: import::EVERYDAY_ACCOUNT_ID,
                        bookings,
                    },
                )
            });
            let (kind, text) = match committed {
                Ok(committed) => {
                    self.close();
                    cx.emit(ImportCommitted);
                    (
                        ToastKind::Success,
                        crate::msg::desktop_import_done(
                            i64::try_from(committed.transactions).unwrap_or(i64::MAX),
                        ),
                    )
                }
                Err(refusal) => (
                    ToastKind::Error,
                    lib_payees::presenter::refusal_text(&refusal),
                ),
            };
            Queue.notify(Notice::Toast(kind, text));
        });
        cx.notify();
    }

    /// A click on a row selects it, closing another row's open select.
    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.state.as_mut() {
            state.selected = index;
            if state
                .open_select
                .as_ref()
                .is_some_and(|(row, _, _)| *row != index)
            {
                state.open_select = None;
            }
        }
        cx.notify();
    }

    /// A click on a row's select: opens it, or closes it when it is already the open one.
    pub(super) fn click_select(
        &mut self,
        index: usize,
        select: RowSelect,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.categories(cx);
        let payees = self.payees(cx).to_vec();
        if let Some(state) = self.state.as_mut() {
            if state
                .open_select
                .as_ref()
                .is_some_and(|(row, open, _)| *row == index && *open == select)
            {
                state.open_select = None;
            } else if let Some(row) = state.rows.get(index) {
                let choices = super::payee_choices(&payees, &row.raw);
                state.open(index, select, &choices, &categories);
            }
        }
        cx.notify();
    }

    pub(super) fn click_option(&mut self, option: usize, cx: &mut Context<'_, Self>) {
        let categories = self.categories(cx);
        let payees = self.payees(cx).to_vec();
        if let Some(state) = self.state.as_mut()
            && let Some((index, _, _)) = state.open_select.as_ref()
            && let Some(row) = state.rows.get(*index)
        {
            let choices = super::payee_choices(&payees, &row.raw);
            state.choose(option, &payees, &choices, &categories);
        }
        cx.notify();
    }

    pub(super) fn click_remember(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.state.as_mut() {
            state.remember = !state.remember;
        }
        cx.notify();
    }

    pub(super) fn click_back(&mut self, cx: &mut Context<'_, Self>) {
        self.close();
        cx.notify();
    }

    /// The status line's legend, and how many rows still need review.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let state = self.state.as_ref()?;
        let pending = import::summary(&state.rows).needs_review;
        Some(PageStatus {
            hints: owned_hints(import_hints()),
            right: if pending == 0 {
                crate::msg::desktop_import_status_ready()
            } else {
                crate::msg::desktop_import_status_pending(
                    i64::try_from(pending).unwrap_or(i64::MAX),
                )
            },
        })
    }
}
