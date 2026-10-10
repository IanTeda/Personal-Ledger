//! The Transactions destination's wiring: keys, clicks, the search, the filter popover, the Import flow and the row opener. The pure rules live in `transactions`; the chrome in `view::transactions`, and its View state in the `TransactionsView` Entity.

use super::Shell;

use crate::{
    accounts::{self},
    import::{self, ImportState, RowSelect},
    navigation::key_router::Movement,
    navigation::nav::{FocusZone, InputMode, Noun},
    payees::{self},
    transactions::{
        self, Transaction,
        chips::FilterField,
        edit_transactions,
        filter_form::{FilterForm, FormField, FormOptions, SelectKey as FilterSelectKey},
        query::{Ledger, TransactionFilters},
        rows::DisplayPrefs,
    },
    view::{format, import as import_view, transactions::state::TransactionsState},
};

use gpui::{App, Context, Keystroke, ScrollStrategy};
use lib_core::DateStyle;
use lib_toast::ToastKind;

impl Shell {
    /// The Transactions, read through their store (ADR-0032).
    pub(super) fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.transactions_store.read(cx).transactions()
    }

    /// The Transactions page's state, read through its Entity (ADR-0032).
    pub(super) fn transactions_state<'a>(&self, cx: &'a App) -> &'a TransactionsState {
        self.transactions_view.read(cx).state()
    }

    /// Edits the Transactions page's state through its Entity, which notifies after the edit.
    pub(super) fn edit_transactions_state<R>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut TransactionsState) -> R,
    ) -> R {
        self.transactions_view
            .update(cx, |view, cx| view.edit(cx, change))
    }

    /// The Import step's state, read through the Transactions Entity (ADR-0032). `None` while it
    /// is not showing.
    pub(super) fn import_state<'a>(&self, cx: &'a App) -> Option<&'a ImportState> {
        self.transactions_state(cx).import.as_ref()
    }

    /// Edits the Import step's slot through the Transactions Entity, which notifies after the edit.
    pub(super) fn edit_import<R>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut Option<ImportState>) -> R,
    ) -> R {
        self.edit_transactions_state(cx, |state| change(&mut state.import))
    }

    /// Runs `change` on the open filter popover's draft, if there is one.
    pub(super) fn with_filter_form<R>(
        &self,
        cx: &mut App,
        change: impl FnOnce(&mut FilterForm) -> R,
    ) -> Option<R> {
        self.edit_transactions_state(cx, |state| state.filter_form.as_mut().map(change))
    }

    /// The reference data the Transactions engine reads, borrowed from the shared stubs.
    pub(super) fn transactions_ledger<'a>(&'a self, cx: &'a App) -> Ledger<'a> {
        Ledger {
            accounts: self.accounts.read(cx).accounts(),
            categories: &self.categories,
            payees: &self.payees,
            tags: self.tags_list(cx),
        }
    }

    /// The Display preferences the table's text is formatted with.
    pub(super) fn transactions_prefs(&self) -> DisplayPrefs {
        DisplayPrefs {
            date_style: self.settings_date_style,
            glyphs: self.settings_status_glyphs,
        }
    }

    /// How many rows the current filters and search leave visible.
    pub(super) fn transactions_visible_len(&self, cx: &App) -> usize {
        transactions::query::query(
            &self.transactions_ledger(cx),
            self.transactions(cx),
            &self.transactions_state(cx).filters,
            &self.transactions_state(cx).search,
        )
        .rows
        .len()
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` move the table's row selection and keep it in view;
    /// `Enter` would open the transaction, which the bundle designs no screen for.
    ///
    /// The scroll strategy follows the research note: a non-strict `scroll_to_item` applies its
    /// strategy only when the row was off-screen, so stepping down uses `Bottom` and stepping up
    /// `Top` (the new row lands at the edge it came from), and jumps use `Center`.
    pub(super) fn apply_transactions_movement(
        &mut self,
        movement: Movement,
        cx: &mut Context<'_, Self>,
    ) {
        let len = self.transactions_visible_len(cx);
        if len == 0 {
            return;
        }
        let selected =
            transactions::rows::clamp_selection(self.transactions_state(cx).selected, len);
        let last = len - 1;
        let viewport = f32::from(
            self.transactions_state(cx)
                .scroll
                .0
                .borrow()
                .base_handle
                .bounds()
                .size
                .height,
        );
        let half = transactions::rows::half_page_rows(
            viewport,
            format::row_height_px(self.settings_row_density),
        );
        let (next, strategy) = match movement {
            Movement::Next => ((selected + 1).min(last), ScrollStrategy::Bottom),
            Movement::Prev => (selected.saturating_sub(1), ScrollStrategy::Top),
            Movement::First => (0, ScrollStrategy::Top),
            Movement::Last => (last, ScrollStrategy::Bottom),
            Movement::HalfPageDown => ((selected + half).min(last), ScrollStrategy::Center),
            Movement::HalfPageUp => (selected.saturating_sub(half), ScrollStrategy::Center),
            Movement::Enter => {
                self.chrome.status_message =
                    Some(crate::msg::desktop_status_open_transaction_not_yet_built());
                return;
            }
        };
        self.edit_transactions_state(cx, |s| s.selected = next);
        self.transactions_state(cx)
            .scroll
            .scroll_to_item(next, strategy);
    }

    /// The Transactions page's `n` and `e` (only while it is the active noun and the view has focus,
    /// in `Normal` mode): the bundle designs no add or edit flow, so both say so.
    pub(super) fn handle_transactions_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.nav.noun() != Noun::Transactions || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        if keystroke.key == "f" {
            self.open_filter_popover(None, cx);
            return true;
        }
        let message = match keystroke.key.as_str() {
            "n" => crate::msg::desktop_status_add_transaction_not_yet_built(),
            "e" => crate::msg::desktop_status_edit_transaction_not_yet_built(),
            _ => return false,
        };
        self.chrome.status_message = Some(message);
        true
    }

    /// Puts the table back on its first row: called whenever the filters or the search change what
    /// is visible, since the old selection no longer points at the same row.
    pub(super) fn reset_transactions_selection(&mut self, cx: &mut App) {
        self.edit_transactions_state(cx, |s| s.selected = 0);
        self.transactions_state(cx)
            .scroll
            .scroll_to_item(0, ScrollStrategy::Top);
    }

    /// `Esc` while searching Transactions: clears the search text as well as leaving the mode (the
    /// map's decision: search is cleared by `Esc` or by emptying the box).
    pub(super) fn cancel_transactions_search(&mut self, cx: &mut App) {
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.reset_transactions_selection(cx);
    }

    /// Keys while `InputMode::Search` is active on the Transactions page: typing filters live,
    /// `Backspace` edits, `Enter` keeps the text and returns to browsing the (filtered) rows, and
    /// `Esc` (handled with the other modes' exit) clears it.
    pub(super) fn handle_transactions_search_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        match keystroke.key.as_str() {
            "enter" => {
                self.nav.exit_mode();
                true
            }
            "backspace" => {
                self.edit_transactions_state(cx, |s| s.search.pop());
                self.reset_transactions_selection(cx);
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text)
                        if text.chars().count() == 1
                            && text.chars().next().is_some_and(|ch| !ch.is_control()) =>
                    {
                        self.edit_transactions_state(cx, |s| s.search.push_str(text));
                        self.reset_transactions_selection(cx);
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// A click on a filter chip (or its `▾`): opens the popover on that chip's field.
    pub(super) fn handle_transactions_chip_click(
        &mut self,
        field: FilterField,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_filter_popover(Some(field), cx);
        cx.notify();
    }

    /// The Account and Category selects' options, read live from the stub data.
    pub(super) fn filter_form_options(&self, cx: &App) -> FormOptions {
        FormOptions::new(
            self.accounts.read(cx).accounts(),
            &self.categories,
            &self.payees,
            self.tags_list(cx),
        )
    }

    /// Opens the filter popover on a draft of the applied filters. A chip focuses its own field
    /// (the date chip focuses From) and the card anchors under it; `f` (no chip) focuses the first
    /// field and anchors under the first chip.
    pub(super) fn open_filter_popover(
        &mut self,
        chip: Option<FilterField>,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_form_options(cx);
        let mut form = FilterForm::from_filters(
            &self.transactions_state(cx).filters,
            &options,
            self.today,
            self.settings_date_style,
        );
        form.focused = chip.map(FormField::for_chip).unwrap_or_default();
        self.edit_transactions_state(cx, |s| {
            s.filter_anchor = chip.unwrap_or(FilterField::Account)
        });
        self.edit_transactions_state(cx, |s| s.filter_form = Some(form));
        self.nav.enter_mode(InputMode::Filter);
    }

    /// Keys while the filter popover is open. `Tab` / `Shift-Tab` move between fields; a focused
    /// select takes `Up` / `Down` / `Enter` / `Space` as the shared dropdown does; the Status control
    /// steps with `Left` / `Right` (or `Space`); text fields take typing and `Backspace`; `Enter`
    /// applies (from a select it opens or commits the list instead, so `Tab` off it first); `Ctrl-r`
    /// resets the draft. `Esc` never reaches here: it is handled with the other modes' exit.
    pub(super) fn handle_filter_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let options = self.filter_form_options(cx);
        let (today, date_style) = (self.today, self.settings_date_style);
        let Some((handled, apply)) = self.with_filter_form(cx, |form| {
            filter_key(form, keystroke, &options, today, date_style)
        }) else {
            return false;
        };
        if apply {
            self.apply_filter_form(cx);
        }
        handled
    }

    /// **apply**: commits the draft to the applied filters, closes the popover and puts the table
    /// back on its first row. A no-op while a date does not parse.
    pub(super) fn apply_filter_form(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.filter_form_options(cx);
        let Some(filters) = self
            .transactions_state(cx)
            .filter_form
            .as_ref()
            .and_then(|form| form.to_filters(&options, self.today, self.settings_date_style))
        else {
            return;
        };
        self.edit_transactions_state(cx, |s| s.filters = filters);
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.nav.exit_mode();
        self.reset_transactions_selection(cx);
    }

    /// A click on a popover field: a text field takes focus; a select takes focus and toggles its
    /// list.
    pub(super) fn handle_filter_field_click(
        &mut self,
        field: FormField,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_form_options(cx);
        self.with_filter_form(cx, |form| {
            if field.is_select() {
                form.click_select(field, &options);
            } else {
                form.focus(field);
            }
        });
        cx.notify();
    }

    pub(super) fn handle_filter_option_click(
        &mut self,
        field: FormField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_form_options(cx);
        self.with_filter_form(cx, |form| form.choose_option(field, index, &options));
        cx.notify();
    }

    pub(super) fn handle_filter_status_click(
        &mut self,
        status: transactions::query::StatusFilter,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_filter_form(cx, |form| {
            form.focus(FormField::Status);
            form.status = status;
        });
        cx.notify();
    }

    /// `reset`: the draft back to the defaults; the applied filters are untouched.
    pub(super) fn handle_filter_reset(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.filter_form_options(cx);
        let (today, date_style) = (self.today, self.settings_date_style);
        self.with_filter_form(cx, |form| form.reset(&options, today, date_style));
        cx.notify();
    }

    pub(super) fn handle_filter_apply(&mut self, cx: &mut Context<'_, Self>) {
        self.apply_filter_form(cx);
        cx.notify();
    }

    /// A click outside the card: discards the draft, like `Esc`.
    pub(super) fn handle_filter_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.nav.exit_mode();
        cx.notify();
    }

    /// The `✕` on an accent chip: resets just that filter.
    pub(super) fn handle_transactions_chip_clear(
        &mut self,
        field: FilterField,
        cx: &mut Context<'_, Self>,
    ) {
        let today = self.today;
        self.edit_transactions_state(cx, |s| {
            transactions::chips::clear_field(&mut s.filters, field, today)
        });
        self.reset_transactions_selection(cx);
        cx.notify();
    }

    /// `clear filters`: every filter back to its default. The search text is separate state and is
    /// left alone.
    pub(super) fn handle_transactions_clear_all(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_transactions_state(cx, |s| s.filters = TransactionFilters::defaults(self.today));
        self.reset_transactions_selection(cx);
        cx.notify();
    }

    /// The header's **add transaction** button: the same message `n` gives.
    pub(super) fn handle_transactions_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_add_transaction_not_yet_built());
        cx.notify();
    }

    /// A click on the search box: the same as pressing `/`.
    pub(super) fn handle_transactions_search_click(&mut self, cx: &mut Context<'_, Self>) {
        self.nav.enter_mode(InputMode::Search);
        cx.notify();
    }

    /// A click on a table row selects it.
    pub(super) fn handle_transactions_row_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.edit_transactions_state(cx, |s| s.selected = index);
        cx.notify();
    }

    /// The Transactions page with `transaction_id` selected, the date range widened to reach it
    /// when it falls outside this year's. A no-op when the Transaction has gone.
    pub(super) fn open_transaction_row(&mut self, transaction_id: u32, cx: &mut Context<'_, Self>) {
        let Some(date) = self
            .transactions(cx)
            .iter()
            .find(|t| t.id == transaction_id)
            .map(|t| t.date)
        else {
            return;
        };
        let mut filters = TransactionFilters::defaults(self.today);
        filters.from = filters.from.map(|from| from.min(date));
        filters.to = filters.to.map(|to| to.max(date));
        self.edit_transactions_state(cx, |s| s.filters = filters);
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        let index = transactions::query::query(
            &self.transactions_ledger(cx),
            self.transactions(cx),
            &self.transactions_state(cx).filters,
            &self.transactions_state(cx).search,
        )
        .rows
        .iter()
        .position(|row| row.transaction.id == transaction_id)
        .unwrap_or(0);
        self.edit_transactions_state(cx, |s| s.selected = index);
        self.transactions_state(cx)
            .scroll
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// The Default category select's options: "none", then the leaf Categories.
    /// `:import`: opens the stubbed 6e step on the seeded statement, in place of the Transactions
    /// page (#284).
    pub(super) fn open_import(&mut self, cx: &mut App) {
        let import = ImportState::new(&self.payees, self.today);
        self.edit_transactions_state(cx, |s| {
            s.import = Some(import);
            s.filter_form = None;
        });
        self.nav.set_noun(Noun::Transactions);
        self.nav.set_focus(FocusZone::View);
        self.reset_view_scroll(cx);
    }

    /// The Category select's options on 6e: "choose category…", then every leaf.
    pub(super) fn import_category_options(&self) -> payees::form::PayeeOptions {
        payees::form::PayeeOptions::new(
            &self.categories,
            crate::msg::desktop_import_choose_category(),
        )
    }

    /// 6e's keys, while it shows with the view focused in `Normal` mode: `j`/`k` move the row, `p`
    /// and `c` open its Payee and Category selects, `n` creates a new Payee from its cleaned name,
    /// `r` toggles "remember", `enter` continues and `esc` goes back. While a select is open it
    /// owns `j`/`k`/arrows, `enter`/`space` and `esc`. Any other key falls through to the router.
    pub(super) fn handle_import_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.import_state(cx).is_none()
            || self.nav.noun() != Noun::Transactions
            || self.nav.mode() != InputMode::Normal
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        let categories = self.import_category_options();
        let key = keystroke.key.as_str();
        let Some(state) = self.import_state(cx) else {
            return false;
        };
        let len = state.rows.len();
        let selected = state.selected.min(len.saturating_sub(1));
        let select_open = state.open_select.is_some();
        let choices = state
            .rows
            .get(selected)
            .map(|row| import_view::payee_choices(&self.payees, &row.raw));
        let Some(choices) = choices else {
            return false;
        };

        if select_open {
            let select_key = match key {
                "j" | "down" => import::SelectKey::Down,
                "k" | "up" => import::SelectKey::Up,
                "enter" | "space" => import::SelectKey::Commit,
                "escape" => import::SelectKey::Cancel,
                // An open list swallows everything else, like the dialogs' selects.
                _ => return true,
            };
            self.edit_import(cx, |import| {
                if let Some(state) = import.as_mut() {
                    state.handle_select_key(select_key, &self.payees, &choices, &categories);
                }
            });
            return true;
        }

        match key {
            "enter" => {
                self.continue_import(cx);
                return true;
            }
            "escape" => {
                self.edit_import(cx, |import| *import = None);
                return true;
            }
            _ => {}
        }
        let handled = self.edit_import(cx, |import| {
            let Some(state) = import.as_mut() else {
                return false;
            };
            match key {
                "j" | "down" => state.selected = accounts::step_selection(selected, len, 1),
                "k" | "up" => state.selected = accounts::step_selection(selected, len, -1),
                "p" => state.open(selected, RowSelect::Payee, &choices, &categories),
                "c" => state.open(selected, RowSelect::Category, &choices, &categories),
                "n" => state.create_new_payee(&self.payees),
                "r" => state.remember = !state.remember,
                _ => return false,
            }
            true
        });
        if handled && let Some(state) = self.import_state(cx) {
            self.view_scroll_handle.scroll_to_item(state.selected);
        }
        handled
    }

    /// **continue** / `enter`: commits the import to the stubs and lands on Transactions with a
    /// Toast. Does nothing while a row needs review (the button is disabled then).
    pub(super) fn continue_import(&mut self, cx: &mut Context<'_, Self>) {
        // Cloned so the commit can borrow `cx` mutably for the store write.
        let Some(state) = self.import_state(cx).cloned() else {
            return;
        };
        let committed = edit_transactions(&self.transactions_store, cx, |transactions| {
            import::commit(
                &state.rows,
                state.remember,
                import::EVERYDAY_ACCOUNT_ID,
                &mut self.payees,
                transactions,
            )
        });
        match committed {
            Ok(committed) => {
                let imported = u32::try_from(committed.transactions).unwrap_or(u32::MAX);
                self.accounts.update(cx, |store, cx| {
                    store.mutate(cx, |service| {
                        if let Some(account) = service.get_mut(import::EVERYDAY_ACCOUNT_ID) {
                            account.transaction_count =
                                account.transaction_count.saturating_add(imported);
                        }
                    })
                });
                self.edit_import(cx, |import| *import = None);
                self.reset_transactions_selection(cx);
                self.reset_view_scroll(cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_import_done(
                        i64::try_from(committed.transactions).unwrap_or(i64::MAX),
                    ),
                );
            }
            Err(import::ImportError::NeedsReview(_)) => {}
            Err(import::ImportError::Payee(error)) => {
                self.raise_toast(ToastKind::Error, error.to_string());
            }
        }
    }

    pub(super) fn handle_import_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.edit_import(cx, |import| {
            if let Some(state) = import.as_mut() {
                state.selected = index;
                if state
                    .open_select
                    .as_ref()
                    .is_some_and(|(row, _, _)| *row != index)
                {
                    state.open_select = None;
                }
            }
        });
        self.nav.set_focus(FocusZone::View);
        cx.notify();
    }

    /// A click on a row's select: opens it, or closes it when it is already the open one.
    pub(super) fn handle_import_select_click(
        &mut self,
        index: usize,
        select: RowSelect,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.import_category_options();
        self.edit_import(cx, |import| {
            if let Some(state) = import.as_mut() {
                if state
                    .open_select
                    .as_ref()
                    .is_some_and(|(row, open, _)| *row == index && *open == select)
                {
                    state.open_select = None;
                } else if let Some(row) = state.rows.get(index) {
                    let choices = import_view::payee_choices(&self.payees, &row.raw);
                    state.open(index, select, &choices, &categories);
                }
            }
        });
        self.nav.set_focus(FocusZone::View);
        cx.notify();
    }

    pub(super) fn handle_import_option_click(&mut self, option: usize, cx: &mut Context<'_, Self>) {
        let categories = self.import_category_options();
        self.edit_import(cx, |import| {
            if let Some(state) = import.as_mut()
                && let Some((index, _, _)) = state.open_select.as_ref()
                && let Some(row) = state.rows.get(*index)
            {
                let choices = import_view::payee_choices(&self.payees, &row.raw);
                state.choose(option, &self.payees, &choices, &categories);
            }
        });
        cx.notify();
    }

    pub(super) fn handle_import_remember_click(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_import(cx, |import| {
            if let Some(state) = import.as_mut() {
                state.remember = !state.remember;
            }
        });
        cx.notify();
    }

    pub(super) fn handle_import_back_click(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_import(cx, |import| *import = None);
        cx.notify();
    }

    pub(super) fn handle_import_continue_click(&mut self, cx: &mut Context<'_, Self>) {
        self.continue_import(cx);
        cx.notify();
    }

    pub(super) fn open_category_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_category(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Payees page: Transactions filtered to exactly this Payee's id, not
    /// a name substring that would over-match ("BP").
    pub(super) fn open_payee_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_payee(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Tags page: Transactions filtered to exactly this Tag's id, not a
    /// name substring that would over-match ("trip").
    pub(super) fn open_tag_transactions(&mut self, id: u32, cx: &mut App) {
        self.edit_transactions_state(cx, |s| {
            s.filters = TransactionFilters::for_tag(self.today, id)
        });
        self.edit_transactions_state(cx, |s| s.search.clear());
        self.edit_transactions_state(cx, |s| s.filter_form = None);
        self.reset_transactions_selection(cx);
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }
}

/// Re-points every Split on category `from` to `to`, returning how many moved. Runs on the
/// Transactions store, so it borrows no Category or Budget state.
pub(super) fn move_category_splits(transactions: &mut [Transaction], from: u32, to: u32) -> i64 {
    let mut moved = 0_i64;
    for split in transactions.iter_mut().flat_map(|t| t.splits.iter_mut()) {
        if split.category_id == from {
            split.category_id = to;
            moved += 1;
        }
    }
    moved
}

/// The popover's key table, applied to the draft. Returns whether the key was handled, and whether
/// it asks for **apply** (which the caller runs once the draft is no longer borrowed).
fn filter_key(
    form: &mut FilterForm,
    keystroke: &Keystroke,
    options: &FormOptions,
    today: chrono::NaiveDate,
    date_style: Option<DateStyle>,
) -> (bool, bool) {
    let modifiers = &keystroke.modifiers;
    if modifiers.control && keystroke.key == "r" {
        form.reset(options, today, date_style);
        return (true, false);
    }
    match keystroke.key.as_str() {
        "tab" => form.cycle_focus(modifiers.shift, options),
        "up" => {
            form.handle_select_key(FilterSelectKey::Up, options);
        }
        "down" => {
            form.handle_select_key(FilterSelectKey::Down, options);
        }
        "left" if form.focused == FormField::Status => form.step_status(-1),
        "right" if form.focused == FormField::Status => form.step_status(1),
        "space" if form.focused.is_select() => {
            form.handle_select_key(FilterSelectKey::Activate, options);
        }
        "space" if form.focused == FormField::Status => form.step_status(1),
        "enter" if form.focused.is_select() => {
            form.handle_select_key(FilterSelectKey::Activate, options);
        }
        "enter" => return (true, true),
        "backspace" => form.backspace(),
        _ => {
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return (false, false);
            }
            if let Some(text) = keystroke.key_char.as_deref()
                && text.chars().count() == 1
                && let Some(ch) = text.chars().next()
            {
                form.push_char(ch);
            }
        }
    }
    (true, false)
}
