//! The Transactions destination's Entity (ADR-0032, ADR-0037). [`TransactionsView`] owns the page's
//! own state: the selected row, the table's scroll, the filters, the search, the filter popover's
//! draft and the Import step that takes the page over while it is open. It reads the rows through
//! `LedgerStore`, and the Accounts, Categories, Payees and Tags its chips, filter choices
//! and Import selects draw on through theirs. It renders itself (`impl Render` in `super`), handles
//! its own keys, `Esc`, Search and movement, and reaches `Shell` only through [`ViewEvent`].

use chrono::NaiveDate;
use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, ScrollStrategy,
    UniformListScrollHandle,
};

use lib_core::DateStyle;
use lib_list_cursor::Step;

use super::{
    header::ChipBounds,
    hints::{filter_hints, transactions_hints},
};
use crate::{
    chrome::statusline::PageStatus,
    import::ImportState,
    ledger_store::LedgerStore,
    navigation::key_router::Movement,
    payees::{Payee, PayeesStore},
    settings::display::{RowDensity, StatusGlyphs},
    transactions::{
        self, Transaction,
        chips::FilterField,
        filter_form::{FilterForm, FormField, FormOptions, SelectKey as FilterSelectKey},
        query::{Ledger, StatusFilter, TransactionFilters},
        rows::DisplayPrefs,
    },
    view::{
        categories::state::CategoriesStore,
        event::{Reveal, ViewEvent, ViewMode},
        format,
        import::import_hints,
        tags::state::TagsStore,
    },
};

impl EventEmitter<ViewEvent> for TransactionsView {}

/// The Transactions page's own state, owned by its Entity.
pub struct TransactionsState {
    /// The selected row, a position in the filtered list; clamped wherever it is read.
    pub selected: usize,
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    pub scroll: UniformListScrollHandle,
    /// What the table is filtered by; starts at the defaults (this year, everything else empty).
    pub filters: TransactionFilters,
    /// The search box's text, separate from the filters.
    pub search: String,
    /// The filter popover's draft, `Some` while it is open (`InputMode::Filter`, which the View
    /// entered). Kept apart from [`Self::filters`] until **apply**.
    pub filter_form: Option<FilterForm>,
    /// The chip that opened the popover, which it anchors under.
    pub filter_anchor: FilterField,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    pub chip_bounds: ChipBounds,
    /// The stubbed 6e Import "match payees" step, `Some` while it shows in place of the page
    /// (`:import`). Dropped when the page becomes the destination again.
    pub import: Option<ImportState>,
}

impl TransactionsState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            selected: 0,
            scroll: UniformListScrollHandle::new(),
            filters: TransactionFilters::defaults(today),
            search: String::new(),
            filter_form: None,
            filter_anchor: FilterField::Account,
            chip_bounds: ChipBounds::default(),
            import: None,
        }
    }
}

/// The store Entities the Transactions page reads, and the Ledger an Import commits through.
#[derive(Clone)]
pub struct TransactionsSources {
    pub ledger: Entity<LedgerStore>,
    pub categories: Entity<CategoriesStore>,
    pub payees: Entity<PayeesStore>,
    pub tags: Entity<TagsStore>,
}

/// The Transactions Entity.
pub struct TransactionsView {
    pub(super) sources: TransactionsSources,
    pub(super) today: NaiveDate,
    /// The Display Preferences the table's text and row height follow. Pushed by `Shell`, which
    /// owns the Settings page that changes them.
    pub(super) date_style: Option<DateStyle>,
    pub(super) glyphs: StatusGlyphs,
    pub(super) density: RowDensity,
    /// Whether the View's Search is open (`InputMode::Search`, which the View entered).
    pub(super) searching: bool,
    /// The Import step's own scroll, reset as it opens and as the page becomes the destination.
    pub(super) import_scroll: ScrollHandle,
    pub(super) state: TransactionsState,
}

impl TransactionsView {
    pub fn new(sources: TransactionsSources, today: NaiveDate) -> Self {
        Self {
            sources,
            today,
            date_style: None,
            glyphs: StatusGlyphs::default(),
            density: RowDensity::default(),
            searching: false,
            import_scroll: ScrollHandle::new(),
            state: TransactionsState::new(today),
        }
    }

    pub fn state(&self) -> &TransactionsState {
        &self.state
    }

    /// Whether the Import step shows in place of the page.
    pub fn import_open(&self) -> bool {
        self.state.import.is_some()
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(
        &mut self,
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
        density: RowDensity,
        cx: &mut Context<'_, Self>,
    ) {
        if density != self.density {
            // The table's scroll offset is in pixels, so a new row height would leave it pointing
            // at a different row: re-anchor on the selected one for its next paint.
            self.state
                .scroll
                .scroll_to_item_strict(self.state.selected, ScrollStrategy::Center);
        }
        self.date_style = date_style;
        self.glyphs = glyphs;
        self.density = density;
        cx.notify();
    }

    pub fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.sources.ledger.read(cx).ledger().transactions()
    }

    pub(super) fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.sources.payees.read(cx).payees(cx)
    }

    /// The reference data the Transactions engine reads, borrowed from the shared stores.
    pub(super) fn ledger<'a>(&self, cx: &'a App) -> Ledger<'a> {
        Ledger {
            accounts: self.sources.ledger.read(cx).ledger().accounts(),
            categories: self.sources.categories.read(cx).categories(cx),
            payees: self.payees(cx),
            tags: self.sources.tags.read(cx).tags(cx),
            units: self.sources.ledger.read(cx).ledger().units(),
        }
    }

    /// The Display preferences the table's text is formatted with.
    pub(super) fn prefs(&self) -> DisplayPrefs {
        DisplayPrefs {
            date_style: self.date_style,
            glyphs: self.glyphs,
        }
    }

    /// How many rows the current filters and search leave visible.
    fn visible_len(&self, cx: &App) -> usize {
        transactions::query::query(
            &self.ledger(cx),
            self.transactions(cx),
            &self.state.filters,
            &self.state.search,
        )
        .rows
        .len()
    }

    /// The Account, Category, Payee and Tag selects' options, read live from the stores.
    pub(super) fn filter_options(&self, cx: &App) -> FormOptions {
        let ledger = self.ledger(cx);
        FormOptions::new(
            ledger.accounts,
            ledger.categories,
            ledger.payees,
            ledger.tags,
        )
    }

    /// Keys the page claims before the router: every key but `Esc` while the filter popover is
    /// open (the View entered `InputMode::Filter` for it), and the Import step's keys while it
    /// shows.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" {
            return false;
        }
        if self.state.filter_form.is_some() {
            return self.filter_key(keystroke, cx);
        }
        if self.state.import.is_some() {
            return self.import_key(keystroke, cx);
        }
        false
    }

    /// The page's `f`, `n` and `e`. `f` opens the filter popover; the bundle designs no add or edit
    /// flow, so `n` and `e` say so.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.modifiers.shift {
            return false;
        }
        let message = match keystroke.key.as_str() {
            "f" => {
                self.open_filter_popover(None, cx);
                return true;
            }
            "n" => crate::msg::desktop_status_add_transaction_not_yet_built(),
            "e" => crate::msg::desktop_status_edit_transaction_not_yet_built(),
            _ => return false,
        };
        cx.emit(ViewEvent::SetStatus(Some(message)));
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` move the table's row selection and keep it in view;
    /// `Enter` would open the transaction, which the bundle designs no screen for. The Import step
    /// keeps this movement for the keys it leaves to the router.
    ///
    /// The scroll strategy follows the research note: a non-strict `scroll_to_item` applies its
    /// strategy only when the row was off-screen, so stepping down uses `Bottom` and stepping up
    /// `Top` (the new row lands at the edge it came from), and jumps use `Center`.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let len = self.visible_len(cx);
        if len == 0 {
            return;
        }
        let selected = transactions::rows::clamp_selection(self.state.selected, len);
        let viewport = f32::from(
            self.state
                .scroll
                .0
                .borrow()
                .base_handle
                .bounds()
                .size
                .height,
        );
        let half =
            transactions::rows::half_page_rows(viewport, format::row_height_px(self.density));
        let Some(step) = movement.step() else {
            cx.emit(ViewEvent::SetStatus(Some(
                crate::msg::desktop_status_open_transaction_not_yet_built(),
            )));
            return;
        };
        let strategy = match step {
            Step::Next | Step::Last => ScrollStrategy::Bottom,
            Step::Prev | Step::First => ScrollStrategy::Top,
            Step::HalfPageDown | Step::HalfPageUp => ScrollStrategy::Center,
        };
        let next = lib_list_cursor::step_by(selected, len, step, half);
        self.state.selected = next;
        self.state.scroll.scroll_to_item(next, strategy);
        cx.notify();
    }

    /// `Esc`: the filter popover's open list first, then the popover itself (its draft is
    /// discarded), then the Search (its text is cleared, the map's decision: search is cleared by
    /// `Esc` or by emptying the box), then the Import step's open select, then the step itself.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if let Some(form) = self.state.filter_form.as_mut() {
            if !form.close_open_select() {
                self.close_filter_popover(cx);
            }
            cx.notify();
            return true;
        }
        if self.searching {
            self.state.search.clear();
            self.reset_selection();
            self.stop_search(cx);
            return true;
        }
        if self.state.import.is_some() {
            self.import_escape(cx);
            return true;
        }
        false
    }

    /// `/` opens the Search box; `Shell` enters `InputMode::Search` for it.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.searching = true;
        cx.notify();
        true
    }

    /// Keys while the Search is open: typing filters live, `Backspace` edits, and `Enter` keeps the
    /// text and returns to browsing the (filtered) rows. `Esc` comes back as [`Self::escape`].
    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match keystroke.key.as_str() {
            "enter" => self.stop_search(cx),
            "backspace" => {
                self.state.search.pop();
                self.reset_selection();
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
                        self.state.search.push_str(text);
                        self.reset_selection();
                    }
                    _ => return false,
                }
            }
        }
        cx.notify();
        true
    }

    /// The page has just become the destination. An Import step left open when the page was left
    /// doesn't come back with it, and neither does a Search.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.state.import = None;
        self.import_scroll.set_offset(gpui::Point::default());
        if self.searching {
            self.stop_search(cx);
        }
        cx.notify();
    }

    /// Shows what another View handed off: an Account's ledger (the table filtered to it), or a
    /// Transaction, selected, with the date range widened to reach it when it falls outside this
    /// year's and nothing else filtering it out. A no-op when the Transaction has gone.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::AccountTransactions(account_id) = reveal {
            self.show_filtered(TransactionFilters::for_account(self.today, account_id), cx);
            return;
        }
        let Reveal::Transaction(transaction_id) = reveal else {
            return;
        };
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
        self.state.filters = filters;
        self.state.search.clear();
        self.state.filter_form = None;
        let index = transactions::query::query(
            &self.ledger(cx),
            self.transactions(cx),
            &self.state.filters,
            &self.state.search,
        )
        .rows
        .iter()
        .position(|row| row.transaction.id == transaction_id)
        .unwrap_or(0);
        self.state.selected = index;
        self.state
            .scroll
            .scroll_to_item_strict(index, ScrollStrategy::Center);
        cx.notify();
    }

    /// The status line's legend (the Import step's, the filter popover's or the page's), and its
    /// right-hand text.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        if let Some(import) = self.state.import.as_ref() {
            let pending = crate::import::summary(&import.rows).needs_review;
            return Some(PageStatus {
                hints: import_hints(),
                right: if pending == 0 {
                    crate::msg::desktop_import_status_ready()
                } else {
                    crate::msg::desktop_import_status_pending(
                        i64::try_from(pending).unwrap_or(i64::MAX),
                    )
                },
            });
        }
        Some(PageStatus {
            hints: if self.state.filter_form.is_some() {
                filter_hints()
            } else {
                transactions_hints()
            },
            right: format::status_legend(self.glyphs),
        })
    }

    /// The table filtered to `filters` from its first row, with no search or draft left over: the
    /// "View transactions" links of Accounts, Payees, Tags and the Budgets detail.
    pub fn show_filtered(&mut self, filters: TransactionFilters, cx: &mut Context<'_, Self>) {
        self.state.filters = filters;
        self.state.search.clear();
        self.state.filter_form = None;
        self.reset_selection();
        cx.notify();
    }

    /// Leaves the Search, keeping its text.
    fn stop_search(&mut self, cx: &mut Context<'_, Self>) {
        self.searching = false;
        cx.emit(ViewEvent::ExitMode);
    }

    /// Puts the table back on its first row: called whenever the filters or the search change what
    /// is visible, since the old selection no longer points at the same row.
    pub(super) fn reset_selection(&mut self) {
        self.state.selected = 0;
        self.state.scroll.scroll_to_item(0, ScrollStrategy::Top);
    }

    /// Opens the filter popover on a draft of the applied filters. A chip focuses its own field
    /// (the date chip focuses From) and the card anchors under it; `f` (no chip) focuses the first
    /// field and anchors under the first chip.
    pub(super) fn open_filter_popover(
        &mut self,
        chip: Option<FilterField>,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_options(cx);
        let mut form =
            FilterForm::from_filters(&self.state.filters, &options, self.today, self.date_style);
        form.focused = chip.map(FormField::for_chip).unwrap_or_default();
        self.state.filter_anchor = chip.unwrap_or(FilterField::Account);
        self.state.filter_form = Some(form);
        cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
        cx.notify();
    }

    /// Discards the popover's draft and leaves the mode it opened.
    pub(super) fn close_filter_popover(&mut self, cx: &mut Context<'_, Self>) {
        self.state.filter_form = None;
        cx.emit(ViewEvent::ExitMode);
        cx.notify();
    }

    /// Keys while the filter popover is open. `Tab` / `Shift-Tab` move between fields; a focused
    /// select takes `Up` / `Down` / `Enter` / `Space` as the shared dropdown does; the Status control
    /// steps with `Left` / `Right` (or `Space`); text fields take typing and `Backspace`; `Enter`
    /// applies (from a select it opens or commits the list instead, so `Tab` off it first); `Ctrl-r`
    /// resets the draft. `Esc` comes back as [`Self::escape`].
    fn filter_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let options = self.filter_options(cx);
        let (today, date_style) = (self.today, self.date_style);
        let Some(form) = self.state.filter_form.as_mut() else {
            return false;
        };
        let (handled, apply) = filter_key(form, keystroke, &options, today, date_style);
        if apply {
            self.apply_filter_form(cx);
        }
        cx.notify();
        handled
    }

    /// **apply**: commits the draft to the applied filters, closes the popover and puts the table
    /// back on its first row. A no-op while a date does not parse.
    pub(super) fn apply_filter_form(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.filter_options(cx);
        let Some(filters) = self
            .state
            .filter_form
            .as_ref()
            .and_then(|form| form.to_filters(&options, self.today, self.date_style))
        else {
            return;
        };
        self.state.filters = filters;
        self.close_filter_popover(cx);
        self.reset_selection();
    }

    /// A click on a filter chip (or its `▾`): opens the popover on that chip's field.
    pub(super) fn click_chip(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        self.open_filter_popover(Some(field), cx);
    }

    /// The `✕` on an accent chip: resets just that filter.
    pub(super) fn clear_chip(&mut self, field: FilterField, cx: &mut Context<'_, Self>) {
        transactions::chips::clear_field(&mut self.state.filters, field, self.today);
        self.reset_selection();
        cx.notify();
    }

    /// `clear filters`: every filter back to its default. The search text is separate state and is
    /// left alone.
    pub(super) fn clear_all(&mut self, cx: &mut Context<'_, Self>) {
        self.state.filters = TransactionFilters::defaults(self.today);
        self.reset_selection();
        cx.notify();
    }

    /// The header's **add transaction** button: the same message `n` gives.
    pub(super) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_add_transaction_not_yet_built(),
        )));
    }

    /// A click on the search box: the same as pressing `/`.
    pub(super) fn click_search(&mut self, cx: &mut Context<'_, Self>) {
        self.start_search(cx);
        cx.emit(ViewEvent::EnterMode(ViewMode::Search));
    }

    /// A click on a table row selects it.
    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.state.selected = index;
        cx.notify();
    }

    /// A click on a popover field: a text field takes focus; a select takes focus and toggles its
    /// list.
    pub(super) fn click_filter_field(&mut self, field: FormField, cx: &mut Context<'_, Self>) {
        let options = self.filter_options(cx);
        if let Some(form) = self.state.filter_form.as_mut() {
            if field.is_select() {
                form.click_select(field, &options);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    pub(super) fn click_filter_option(
        &mut self,
        field: FormField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let options = self.filter_options(cx);
        if let Some(form) = self.state.filter_form.as_mut() {
            form.choose_option(field, index, &options);
        }
        cx.notify();
    }

    pub(super) fn click_filter_status(&mut self, status: StatusFilter, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.state.filter_form.as_mut() {
            form.focus(FormField::Status);
            form.status = status;
        }
        cx.notify();
    }

    /// `reset`: the draft back to the defaults; the applied filters are untouched.
    pub(super) fn reset_filter_form(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.filter_options(cx);
        let (today, date_style) = (self.today, self.date_style);
        if let Some(form) = self.state.filter_form.as_mut() {
            form.reset(&options, today, date_style);
        }
        cx.notify();
    }
}

/// The popover's key table, applied to the draft. Returns whether the key was handled, and whether
/// it asks for **apply** (which the caller runs once the draft is no longer borrowed).
fn filter_key(
    form: &mut FilterForm,
    keystroke: &Keystroke,
    options: &FormOptions,
    today: NaiveDate,
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
