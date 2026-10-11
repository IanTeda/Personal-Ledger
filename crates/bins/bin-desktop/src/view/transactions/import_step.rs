//! The stubbed 6e Import step's behaviour on the Transactions Entity (#284): opened by `:import`,
//! it shows in place of the page until it is committed or left. An `impl TransactionsView` block
//! for the concern; the pure rules live in `crate::import` and the drawing in `view::import`.

use gpui::{App, Context, Keystroke};
use lib_ledger::{CommitImport, CountImportedTransactions};
use lib_list_cursor::Step;
use lib_toast::ToastKind;

use super::state::TransactionsView;
use crate::{
    import::{self, ImportState, RowSelect},
    payees::form::PayeeOptions,
    view::{event::ViewEvent, import as import_view},
};

impl TransactionsView {
    /// `:import`: opens the step on the seeded statement, in place of the page.
    pub fn open_import(&mut self, cx: &mut Context<'_, Self>) {
        self.state.import = Some(ImportState::new(self.payees(cx), self.today));
        self.state.filter_form = None;
        self.import_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// The Category select's options on 6e: "choose category…", then every leaf.
    pub(super) fn import_categories(&self, cx: &App) -> PayeeOptions {
        PayeeOptions::new(
            self.sources.categories.read(cx).categories(cx),
            crate::msg::desktop_import_choose_category(),
        )
    }

    /// 6e's keys: `j`/`k` move the row, `p` and `c` open its Payee and Category selects, `n`
    /// creates a new Payee from its cleaned name, `r` toggles "remember" and `enter` continues.
    /// While a select is open it owns `j`/`k`/arrows and `enter`/`space`. Any other key falls
    /// through to the router; `esc` comes back as [`Self::escape`].
    pub(super) fn import_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let categories = self.import_categories(cx);
        let payees = self.payees(cx).to_vec();
        let key = keystroke.key.as_str();
        let Some(state) = self.state.import.as_mut() else {
            return false;
        };
        let len = state.rows.len();
        let selected = state.selected.min(len.saturating_sub(1));
        let Some(choices) = state
            .rows
            .get(selected)
            .map(|row| import_view::payee_choices(&payees, &row.raw))
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
        self.import_scroll.scroll_to_item(selected);
        cx.notify();
        true
    }

    /// `esc` on 6e: closes an open select, else goes back to the page.
    pub(super) fn import_escape(&mut self, cx: &mut Context<'_, Self>) {
        let categories = self.import_categories(cx);
        let payees = self.payees(cx).to_vec();
        if let Some(state) = self.state.import.as_mut() {
            if state.open_select.is_some() {
                let choices = state
                    .rows
                    .get(state.selected.min(state.rows.len().saturating_sub(1)))
                    .map(|row| import_view::payee_choices(&payees, &row.raw));
                if let Some(choices) = choices {
                    state.handle_select_key(
                        import::SelectKey::Cancel,
                        &payees,
                        &choices,
                        &categories,
                    );
                }
            } else {
                self.state.import = None;
            }
        }
        cx.notify();
    }

    /// **continue** / `enter`: commits the import to the Ledger and lands on the page with a Toast.
    /// Does nothing while a row needs review (the button is disabled then).
    pub(super) fn continue_import(&mut self, cx: &mut Context<'_, Self>) {
        let Some(state) = self.state.import.as_ref() else {
            return;
        };
        let bookings = match import::bookings(&state.rows, state.remember) {
            Ok(bookings) => bookings,
            Err(import::ImportError::NeedsReview(_)) => return,
        };
        let committed = self.sources.ledger.update(cx, |store, cx| {
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
                let imported = u32::try_from(committed.transactions).unwrap_or(u32::MAX);
                // A missing Everyday account has no count to bump, as before.
                let _ = self.sources.ledger.update(cx, |store, cx| {
                    store.apply(
                        cx,
                        CountImportedTransactions {
                            id: import::EVERYDAY_ACCOUNT_ID,
                            imported,
                        },
                    )
                });
                self.state.import = None;
                self.reset_selection();
                self.import_scroll.set_offset(gpui::Point::default());
                (
                    ToastKind::Success,
                    crate::msg::desktop_import_done(
                        i64::try_from(committed.transactions).unwrap_or(i64::MAX),
                    ),
                )
            }
            Err(refusal) => (ToastKind::Error, refusal.to_string()),
        };
        cx.emit(ViewEvent::RaiseToast { kind, text });
        cx.notify();
    }

    /// A click on a row selects it, closing another row's open select.
    pub(super) fn click_import_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.state.import.as_mut() {
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
    pub(super) fn click_import_select(
        &mut self,
        index: usize,
        select: RowSelect,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.import_categories(cx);
        let payees = self.payees(cx).to_vec();
        if let Some(state) = self.state.import.as_mut() {
            if state
                .open_select
                .as_ref()
                .is_some_and(|(row, open, _)| *row == index && *open == select)
            {
                state.open_select = None;
            } else if let Some(row) = state.rows.get(index) {
                let choices = import_view::payee_choices(&payees, &row.raw);
                state.open(index, select, &choices, &categories);
            }
        }
        cx.notify();
    }

    pub(super) fn click_import_option(&mut self, option: usize, cx: &mut Context<'_, Self>) {
        let categories = self.import_categories(cx);
        let payees = self.payees(cx).to_vec();
        if let Some(state) = self.state.import.as_mut()
            && let Some((index, _, _)) = state.open_select.as_ref()
            && let Some(row) = state.rows.get(*index)
        {
            let choices = import_view::payee_choices(&payees, &row.raw);
            state.choose(option, &payees, &choices, &categories);
        }
        cx.notify();
    }

    pub(super) fn click_import_remember(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(state) = self.state.import.as_mut() {
            state.remember = !state.remember;
        }
        cx.notify();
    }

    pub(super) fn click_import_back(&mut self, cx: &mut Context<'_, Self>) {
        self.state.import = None;
        cx.notify();
    }
}
